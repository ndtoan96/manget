use log::warn;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crate::download::{download, DownloadError, DownloadItem, DownloadOptions, DownloadSpeedLimit};

pub trait Chapter {
    fn url(&self) -> &str;
    fn title(&self) -> &str;
    fn chapter_name(&self) -> Option<&str>;
    fn pages_download_info(&self) -> &Vec<DownloadItem>;
    fn server_speed_limit(&self) -> Option<DownloadSpeedLimit>;
}

#[derive(Debug, thiserror::Error)]
pub enum ChapterDownloadError {
    #[error("cannot download to {path}")]
    PathError {
        path: PathBuf,
        source: DownloadError,
    },
    #[error("failed to download some pages")]
    PagesDownloadError { sources: Vec<DownloadError> },
}

pub async fn download_chapter(
    chapter: &impl Chapter,
    path: Option<&Path>,
) -> Result<(), ChapterDownloadError> {
    let download_path = path
        .map(|x| x.to_path_buf())
        .unwrap_or(Path::new(".").join(&generate_folder_name(chapter)));
    let mut options = DownloadOptions::new()
        .set_path(&download_path)
        .map_err(|e| ChapterDownloadError::PathError {
            path: download_path.to_path_buf(),
            source: e,
        })?;
    if let Some(limit) = chapter.server_speed_limit() {
        options.set_limit_speed(limit);
    }

    options.add_download_items(chapter.pages_download_info());

    let mut failed_items = Vec::new();

    for result in download(&options).await {
        match result {
            Err(DownloadError::RequestError { item, source: _ }) => {
                failed_items.push(item);
            }
            _ => (),
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
            Ok(())
        } else {
            let mut sources = Vec::new();
            for result in results {
                if let Err(e) = result {
                    sources.push(e);
                }
            }
            Err(ChapterDownloadError::PagesDownloadError { sources })
        }
    } else {
        Ok(())
    }
}

pub fn download_chapter_as_zip(chapter: impl Chapter, zip_path: Option<&Path>) {
    todo!()
}

fn generate_folder_name(chapter: &impl Chapter) -> String {
    format!(
        "{} - {}",
        chapter.title(),
        chapter.chapter_name().unwrap_or("chapter 0")
    )
}
