use log::{info, warn};
use reqwest::IntoUrl;
use std::{
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use zip::write::FileOptions;
use zip::ZipWriter;

use crate::{
    download::{download, DownloadError, DownloadItem, DownloadOptions, DownloadSpeedLimit},
    mangapark,
};

pub trait Chapter {
    fn url(&self) -> &str;
    fn title(&self) -> &str;
    fn chapter_name(&self) -> Option<&str>;
    fn pages_download_info(&self) -> &Vec<DownloadItem>;
    fn server_speed_limit(&self) -> Option<DownloadSpeedLimit>;
}

#[derive(Debug, thiserror::Error)]
pub enum ChapterError {
    #[error("cannot download to {path}")]
    PathError {
        path: PathBuf,
        source: DownloadError,
    },
    #[error("failed to download some pages")]
    PagesDownloadError { sources: Vec<DownloadError> },
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error(transparent)]
    MangaParkError(#[from] mangapark::MangaParkError),
    #[error("site '{0}' is not supported")]
    SiteNotSupported(String),
}

pub async fn download_chapter<P: Into<PathBuf>>(
    chapter: &impl Chapter,
    path: Option<P>,
) -> Result<PathBuf, ChapterError> {
    let download_path = path
        .map(|x| x.into())
        .unwrap_or(Path::new(".").join(&generate_chapter_full_name(chapter)));
    let mut options = DownloadOptions::new()
        .set_path(&download_path)
        .map_err(|e| ChapterError::PathError {
            path: download_path.to_path_buf(),
            source: e,
        })?;
    if let Some(limit) = chapter.server_speed_limit() {
        options.set_limit_speed(limit);
    }

    options.add_download_items(chapter.pages_download_info());

    let mut failed_items = Vec::new();

    for result in download(&options).await {
        if let Err(DownloadError::RequestError { item, source: _ }) = result {
            failed_items.push(item);
        }
    }

    // retry failed items after some time
    if !failed_items.is_empty() {
        warn!("** some download items have failed, wait for 5 seconds and retry **");
        tokio::time::sleep(Duration::from_secs(5)).await;
        options.clear_download_items();
        options.add_download_items(&failed_items);
        let results = download(&options).await;
        if results.iter().all(|x| x.is_ok()) {
            Ok(download_path)
        } else {
            let mut sources = Vec::new();
            for result in results {
                if let Err(e) = result {
                    sources.push(e);
                }
            }
            Err(ChapterError::PagesDownloadError { sources })
        }
    } else {
        Ok(download_path)
    }
}

pub async fn download_chapter_as_cbz<P: Into<PathBuf>>(
    chapter: &impl Chapter,
    zip_path: Option<P>,
) -> Result<PathBuf, ChapterError> {
    let tempdir = tempfile::tempdir()?;
    let outdir = download_chapter(chapter, Some(tempdir.into_path())).await?;
    let zip_path = zip_path.map(|p| p.into()).unwrap_or(
        PathBuf::from(".")
            .join(generate_chapter_full_name(chapter))
            .with_extension("cbz"),
    );
    if let Some(p) = zip_path.parent() {
        fs::create_dir_all(p)?;
    }
    info!("Compressing...");
    zip_folder(&outdir, &zip_path)?;
    info!("Done.");
    Ok(zip_path)
}

pub fn generate_chapter_full_name(chapter: &impl Chapter) -> String {
    format!(
        "{} - {}",
        chapter.title(),
        chapter.chapter_name().unwrap_or("chapter 0")
    )
}

pub async fn get_chapter(
    url: impl IntoUrl + Display + Clone,
) -> Result<impl Chapter, ChapterError> {
    let url = url
        .clone()
        .into_url()
        .map_err(|_| ChapterError::InvalidUrl(url.to_string()))?;
    match url.domain() {
        Some("mangapark.net") => Ok(mangapark::MangaParkChapter::from(url).await?),
        Some(x) => Err(ChapterError::SiteNotSupported(x.to_string())),
        None => Err(ChapterError::InvalidUrl(url.to_string())),
    }
}

fn zip_folder<P: Into<PathBuf>>(
    folder_path: P,
    zip_path: P,
) -> std::result::Result<(), std::io::Error> {
    let folder_path = folder_path.into();
    let output_path = zip_path.into();
    let file: fs::File = fs::File::create(&output_path)?;
    let writer = std::io::BufWriter::new(file);
    let mut zip = ZipWriter::new(writer);

    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let files = fs::read_dir(&folder_path)?;
    for file in files {
        let file = file?;
        let path = file.path();

        if path.is_file() {
            let relative_path = path.strip_prefix(&folder_path).unwrap();
            zip.start_file(relative_path.to_str().unwrap(), options)?;
            let mut source_file = fs::File::open(path)?;
            std::io::copy(&mut source_file, &mut zip)?;
        }
    }

    zip.finish()?;
    Ok(())
}