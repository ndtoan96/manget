use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Json;
use manget::manga;
use manget::manga::ChapterError;
use serde::Deserialize;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct DownloadRequest {
    url: String,
}

async fn download_chapter_from_url(url: String) -> Result<(String, PathBuf), ChapterError> {
    let chapter = manga::get_chapter(url).await?;
    let random_file_name = Uuid::new_v4().to_string();
    let zip_path = tempfile::tempdir()?.into_path().join(random_file_name);
    let file_path = manga::download_chapter_as_cbz(&chapter, Some(zip_path)).await?;
    let chapter_full_name = manga::generate_chapter_full_name(&chapter);
    Ok((format!("{chapter_full_name}.cbz"), file_path))
}

async fn download(Json(payload): Json<DownloadRequest>) -> Response {
    let download_result = download_chapter_from_url(payload.url).await;
    match download_result {
        Ok((name, path)) => match tokio::fs::read(path).await {
            Ok(content) => (
                StatusCode::OK,
                [(
                    axum::http::header::CONTENT_DISPOSITION,
                    format!("attachment; filename={}", name),
                )],
                content,
            )
                .into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(ChapterError::SiteNotSupported(url)) => {
            (StatusCode::BAD_REQUEST, format!("{} is not supported", url)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let app = axum::Router::new()
        .route("/download", post(download))
        .layer(tower_http::trace::TraceLayer::new_for_http());

    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        // .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.unwrap() })
        .await
        .unwrap();
}
