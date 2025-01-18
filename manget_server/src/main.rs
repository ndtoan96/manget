mod novel;

use axum::body::Bytes;
use axum::extract::Request;
use axum::http::header::InvalidHeaderValue;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{debug_handler, Json, Router};
use manget::manga;
use manget::manga::ChapterError;
use sanitize_filename::sanitize;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::ops::Deref;
use std::path::PathBuf;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
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
    #[error("{0}")]
    EpubError(String),
    #[error(transparent)]
    HeaderError(#[from] InvalidHeaderValue),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

#[debug_handler]
async fn novel(
    Json(NovelDownloadRequest { title, content }): Json<NovelDownloadRequest>,
) -> Result<impl IntoResponse, AppError> {
    let data = novel::convert_chapter_html_to_epub(&title, &content)
        .await
        .map_err(|e| AppError::EpubError(e.to_string()))?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename={}.epub", sanitize(title)))?,
    );

    Ok((headers, data))
}

async fn download(json: Json<DownloadRequest>) -> Result<impl IntoResponse, AppError> {
    let (file_name, file_path) = download_chapter_from_url(&json.url).await?;
    let mut data = Vec::new();

    // load file to local variable and delete file on disk
    std::fs::File::open(&file_path)?.read_to_end(&mut data)?;
    let _ = std::fs::remove_file(&file_path);
    if let Some(p) = file_path.parent() {
        let _ = std::fs::remove_dir(p);
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename={}", sanitize(file_name)))?,
    );

    Ok((headers, data))
}

#[derive(Debug, Serialize)]
struct ChapterInfoResponseBody {
    chapter_name: String,
}

async fn chapter_info(json: Json<DownloadRequest>) -> Result<impl IntoResponse, AppError> {
    let chapter = manga::get_chapter(&json.url).await?;
    let chapter_full_name = chapter.full_name();
    let response_body = ChapterInfoResponseBody {
        chapter_name: chapter_full_name.trim().to_string(),
    };
    Ok(Json(response_body))
}

async fn download_chapter_from_url(url: &str) -> Result<(String, PathBuf), ChapterError> {
    let chapter = manga::get_chapter(url).await?;
    let random_file_name = Uuid::new_v4().to_string();
    let zip_path = tempfile::tempdir()?.into_path().join(random_file_name);
    let file_path = manga::download_chapter_as_cbz(chapter.deref(), Some(zip_path)).await?;
    let chapter_full_name = chapter.full_name();
    Ok((format!("{chapter_full_name}.cbz"), file_path))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let app = Router::new()
        .route("/", get(|| async { "Toan's server" }))
        .route("/get_chapter_info", post(chapter_info))
        .route("/download", post(download))
        .route("/novel", post(novel))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
