use actix_cors::Cors;
use actix_web::http::header;
use actix_web::{middleware::Logger, post, App, HttpServer};
use actix_web::{web, HttpResponse, Responder, ResponseError};
use manget::manga;
use manget::manga::ChapterError;
use sanitize_filename::sanitize;
use scraper::Selector;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::ops::Deref;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct DownloadRequest {
    url: String,
}

#[derive(Debug, Deserialize)]
struct NovelDownloadRequest {
    title: String,
    content: String,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error(transparent)]
    Chapter(#[from] manga::ChapterError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("$0")]
    EpubError(String),
}

impl ResponseError for AppError {}

#[post("/novel")]
async fn novel(
    web::Json(NovelDownloadRequest { title, content }): web::Json<NovelDownloadRequest>,
) -> Result<HttpResponse, AppError> {
    let data = convert_chapter_html_to_epub(&title, &content)
        .map_err(|e| AppError::EpubError(e.to_string()))?;
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .append_header(header::ContentDisposition {
            disposition: header::DispositionType::Attachment,
            parameters: vec![header::DispositionParam::Filename(sanitize(format!(
                "{}.epub",
                title
            )))],
        })
        .body(data))
}

#[post("/download")]
async fn download(json: web::Json<DownloadRequest>) -> Result<HttpResponse, AppError> {
    let (file_name, file_path) = download_chapter_from_url(&json.url).await?;
    let mut data = Vec::new();

    // load file to local variable and delete file on disk
    std::fs::File::open(&file_path)?.read_to_end(&mut data)?;
    let _ = std::fs::remove_file(&file_path);
    if let Some(p) = file_path.parent() {
        let _ = std::fs::remove_dir(p);
    }

    // return the data
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .append_header(header::ContentDisposition {
            disposition: header::DispositionType::Attachment,
            parameters: vec![header::DispositionParam::Filename(file_name)],
        })
        .body(data))
}

#[derive(Debug, Serialize)]
struct ChapterInfoResponseBody {
    chapter_name: String,
}

#[post("/get_chapter_info")]
async fn chapter_info(json: web::Json<DownloadRequest>) -> Result<impl Responder, AppError> {
    let chapter = manga::get_chapter(&json.url).await?;
    let chapter_full_name = chapter.full_name();
    let response_body = ChapterInfoResponseBody {
        chapter_name: chapter_full_name.trim().to_string(),
    };
    Ok(web::Json(response_body))
}

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    HttpServer::new(|| {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(["GET", "POST", "OPTIONS"])
            .allowed_headers([header::ACCEPT, header::CONTENT_TYPE]);
        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .service(download)
            .service(chapter_info)
            .service(novel)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

async fn download_chapter_from_url(url: &str) -> Result<(String, PathBuf), ChapterError> {
    let chapter = manga::get_chapter(url).await?;
    let random_file_name = Uuid::new_v4().to_string();
    let zip_path = tempfile::tempdir()?.into_path().join(random_file_name);
    let file_path = manga::download_chapter_as_cbz(chapter.deref(), Some(zip_path)).await?;
    let chapter_full_name = chapter.full_name();
    Ok((format!("{chapter_full_name}.cbz"), file_path))
}

fn convert_chapter_html_to_epub(title: &str, content: &str) -> epub_builder::Result<Vec<u8>> {
    let processed_content = process_chapter_content(content);
    let xhtml = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html>

<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head>
  <title>{title}</title>
</head>

<body>
{processed_content}
</body>
</html>
"#
    );
    let mut output = Vec::new();
    epub_builder::EpubBuilder::new(epub_builder::ZipLibrary::new()?)?
        .metadata("title", title)?
        .epub_version(epub_builder::EpubVersion::V30)
        .add_content(
            epub_builder::EpubContent::new("chapter.xhtml", xhtml.as_bytes())
                .title(title)
                .reftype(epub_builder::ReferenceType::Text),
        )?
        .generate(&mut output)?;
    Ok(output)
}

fn process_chapter_content(content: &str) -> String {
    let html = scraper::Html::parse_fragment(content);
    let selector = Selector::parse(".br-section > *").unwrap();
    let texts: Vec<_> = html
        .select(&selector)
        .filter(|e| e.value().name() != "div")
        .map(|e| e.html())
        .map(|t| {
            if t.starts_with("<img") {
                t.replace(">", "/>")
            } else {
                t
            }
        })
        .collect();
    texts
        .join("\n")
        .replace("<br>", "<br/>")
        .replace("<hr>", "<hr/>")
}
