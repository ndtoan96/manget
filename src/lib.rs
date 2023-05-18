use std::{
    io::{self, Cursor},
    path::{Path, PathBuf},
    time::Duration,
};

use futures::TryFutureExt;
use reqwest::StatusCode;

type Result<T> = std::result::Result<T, DownloaderError>;

#[derive(thiserror::Error, Debug)]
pub enum DownloaderError {
    #[error("path not found: {0}")]
    PathNotFound(PathBuf),
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),
    #[error("{1} - {0}")]
    InvalidRequestStatus(String, StatusCode),
}

pub struct Downloader {
    urls_table: Vec<(String, Option<String>)>,
    speed_limit: Option<(usize, Duration)>,
    path: PathBuf,
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            urls_table: Vec::new(),
            speed_limit: None,
            path: PathBuf::from("."),
        }
    }

    pub fn add_url(&mut self, url: &str) {
        self.urls_table.push((url.to_string(), None));
    }

    pub fn add_url_with_name(&mut self, url: &str, name: &str) {
        self.urls_table
            .push((url.to_string(), Some(name.to_string())));
    }

    pub fn add_urls<'a>(mut self, urls: impl Iterator<Item = &'a str>) {
        urls.for_each(|url| self.urls_table.push((url.to_string(), None)));
    }

    pub fn limit_urls_per_second(self, num_urls: usize) -> Self {
        self.limit_speed(num_urls, Duration::from_secs(1))
    }

    pub fn limit_urls_per_minute(self, num_urls: usize) -> Self {
        self.limit_speed(num_urls, Duration::from_secs(60))
    }

    pub fn limit_speed(mut self, num_urls: usize, every: Duration) -> Self {
        self.speed_limit = Some((num_urls, every));
        self
    }

    pub fn set_path(mut self, path: impl AsRef<Path>) -> Result<Self> {
        if path.as_ref().exists() {
            self.path = path.as_ref().to_owned();
            Ok(self)
        } else {
            Err(DownloaderError::PathNotFound(path.as_ref().to_owned()))
        }
    }

    async fn download_one_url(&self, url: &str, name: &Option<String>) -> Result<PathBuf> {
        let file_name = match name {
            Some(value) => value.to_string(),
            None => reqwest::Url::parse(url)
                .map_err(|_| DownloaderError::InvalidUrl(url.to_string()))?
                .path_segments()
                .ok_or(DownloaderError::InvalidUrl(url.to_string()))?
                .last()
                .ok_or(DownloaderError::InvalidUrl(url.to_string()))?
                .to_string(),
        };
        let file_path = self.path.join(file_name);
        let response = reqwest::get(url).await?;
        if response.status().is_success() {
            let mut file = std::fs::File::create(&file_path)?;
            let mut content = Cursor::new(response.bytes().await?);
            std::io::copy(&mut content, &mut file)?;
            Ok(file_path)
        } else {
            Err(DownloaderError::InvalidRequestStatus(
                url.to_string(),
                response.status(),
            ))
        }
    }

    async fn download_chunk(
        &self,
        url_iter: impl IntoIterator<Item = &(String, Option<String>)>,
    ) -> Vec<Result<PathBuf>> {
        let downloads: Vec<_> = url_iter
            .into_iter()
            .map(|url_and_name| {
                let url = &url_and_name.0;
                let name = &url_and_name.1;
                self.download_one_url(url, name)
                    .and_then(move |p| async {
                        println!("Downloaded: {} -> {}", url.to_string(), p.display());
                        Ok(p)
                    })
                    .or_else(move |e| async {
                        eprintln!("{}", e);
                        Err(e)
                    })
            })
            .collect();
        futures::future::join_all(downloads).await
    }

    pub async fn download(&self) -> Vec<Result<PathBuf>> {
        match self.speed_limit {
            None => self.download_chunk(&self.urls_table).await,
            Some((num_url, duration)) => {
                let mut downloads = Vec::new();
                let mut chunks = self.urls_table.chunks(num_url).peekable();
                while let Some(chunk) = chunks.next() {
                    let mut subdownloads = self.download_chunk(chunk).await;
                    downloads.append(&mut subdownloads);
                    if chunks.peek().is_some() {
                        tokio::time::sleep(duration).await;
                    }
                }
                downloads
            }
        }
    }
}
