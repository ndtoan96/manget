use actix_web::http::header;
use actix_web::{middleware::Logger, post, App, HttpServer};
use actix_web::{web, HttpResponse, ResponseError};
use manget::manga;
use manget::manga::ChapterError;
use serde::Deserialize;
use std::io::Read;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct DownloadRequest {
    url: String,
}

#[derive(Debug, thiserror::Error)]
enum WrapperError {
    #[error(transparent)]
    Chapter(#[from] manga::ChapterError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

impl ResponseError for WrapperError {}

#[post("/download")]
async fn download(json: web::Json<DownloadRequest>) -> Result<HttpResponse, WrapperError> {
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

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    HttpServer::new(|| App::new().wrap(Logger::default()).service(download))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}

async fn download_chapter_from_url(url: &str) -> Result<(String, PathBuf), ChapterError> {
    let chapter = manga::get_chapter(url).await?;
    let random_file_name = Uuid::new_v4().to_string();
    let zip_path = tempfile::tempdir()?.into_path().join(random_file_name);
    let file_path = manga::download_chapter_as_cbz(&chapter, Some(zip_path)).await?;
    let chapter_full_name = manga::generate_chapter_full_name(&chapter);
    Ok((format!("{chapter_full_name}.cbz"), file_path))
}
