use log::{error, info};
use std::{
    fs,
    io::{self, Cursor},
    path::{Path, PathBuf},
};

use futures::FutureExt;
use reqwest::{header::CONTENT_TYPE, Response};

type Result<T> = std::result::Result<T, DownloadError>;

#[derive(thiserror::Error, Debug)]
pub enum DownloadError {
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    ConvertError(#[from] reqwest::header::ToStrError),
    #[error("{source}")]
    RequestError {
        item: DownloadItem,
        source: reqwest::Error,
    },
}

#[derive(Debug, Clone)]
pub struct DownloadItem {
    url: String,
    name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DownloadOptions {
    items: Vec<DownloadItem>,
    path: PathBuf,
    referer: Option<String>,
}

impl DownloadItem {
    pub fn new(url: &str, name: Option<&str>) -> Self {
        Self {
            url: url.to_string(),
            name: name.map(|x| x.to_string()),
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

impl DownloadOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_url(&mut self, url: &str) -> &mut Self {
        self.items.push(DownloadItem {
            url: url.to_string(),
            name: None,
        });
        self
    }

    pub fn add_url_with_name(&mut self, url: &str, name: &str) -> &mut Self {
        self.items.push(DownloadItem::new(url, Some(name)));
        self
    }

    pub fn add_download_item(&mut self, item: &DownloadItem) -> &mut Self {
        self.items.push(item.clone());
        self
    }

    pub fn add_download_items<'a>(&mut self, items: impl IntoIterator<Item = &'a DownloadItem>) -> &mut Self {
        self.items.append(&mut items.into_iter().cloned().collect());
        self
    }

    pub fn add_urls<'a>(mut self, urls: impl Iterator<Item = &'a str>) {
        urls.for_each(|url| self.items.push(DownloadItem::new(url, None)));
    }

    pub fn clear_download_items(&mut self) {
        self.items = Vec::new();
    }

    pub fn set_path(mut self, path: impl AsRef<Path>) -> Result<Self> {
        fs::create_dir_all(&path)?;
        self.path = path.as_ref().to_owned();
        Ok(self)
    }

    pub fn set_referer(&mut self, referer: &str) -> &mut Self {
        self.referer = Some(referer.to_string());
        self
    }
}

pub async fn download(options: &DownloadOptions) -> Vec<Result<PathBuf>> {
    let items = &options.items;
    let path = &options.path;
    let referer = &options.referer;
    let downloads: Vec<_> = items
        .iter()
        .map(|item| {
            download_one_url(item, path, referer).then(|result| async {
                match &result {
                    Ok(p) => info!("Downloaded: {} -> {}", item.url(), p.display()),
                    Err(e) => error!("{e}"),
                }
                result
            })
        })
        .collect();
    futures::future::join_all(downloads).await
}

async fn download_one_url(item: &DownloadItem, path: &Path, referer: &Option<String>) -> Result<PathBuf> {
    let client = reqwest::Client::new();
    let mut request = client.get(&item.url);
    if let Some(r) = referer {
        request = request.header("referer", r);
    }
    let response = request.send()
        .await
        .map_err(|e| DownloadError::RequestError {
            item: item.clone(),
            source: e,
        })?
        .error_for_status()
        .map_err(|e| DownloadError::RequestError {
            item: item.clone(),
            source: e,
        })?;

    // provided file name or inferred from url
    let file_name = match &item.name {
        Some(value) => value.to_string(),
        None => reqwest::Url::parse(&item.url)
            .map_err(|_| DownloadError::InvalidUrl(item.url.to_string()))?
            .path_segments()
            .ok_or(DownloadError::InvalidUrl(item.url.to_string()))?
            .last()
            .ok_or(DownloadError::InvalidUrl(item.url.to_string()))?
            .to_string(),
    };

    // convert to path to check for extension
    let mut file_name = PathBuf::from(file_name);
    if file_name.extension().is_none() {
        if let Some(extension) = infer_extension_from_response(&response) {
            file_name = file_name.with_extension(extension);
        }
    }
    let file_path = path.join(file_name);
    let mut file = std::fs::File::create(&file_path)?;
    let mut content =
        Cursor::new(
            response
                .bytes()
                .await
                .map_err(|e| DownloadError::RequestError {
                    item: item.clone(),
                    source: e,
                })?,
        );
    std::io::copy(&mut content, &mut file)?;
    Ok(file_path)
}

fn infer_extension_from_response(response: &Response) -> Option<String> {
    response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|x| x.to_str().ok())
        .and_then(|x| x.parse::<mime::Mime>().ok())
        .and_then(|x| match x.type_().as_str() {
            "image" => Some(x.subtype().to_string().replace("jpeg", "jpg")),
            "text" => match x.subtype().as_str() {
                "plain" => Some(String::from("txt")),
                "csv" | "html" => Some(x.subtype().to_string()),
                _ => None,
            },
            "application" => match x.subtype().as_str() {
                "pdf" | "json" | "zip" => Some(x.subtype().to_string()),
                _ => None,
            },
            _ => None,
        })
}
