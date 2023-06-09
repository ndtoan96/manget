use std::collections::HashMap;

use log::error;
use reqwest::IntoUrl;
use serde::Deserialize;

use crate::{download::DownloadItem, manga::Chapter};

pub struct MangadexChapter {
    manga_title: String,
    chapter_title: Option<String>,
    chapter: Option<String>,
    volume: Option<String>,
    url: String,
    pages: Vec<DownloadItem>,
}

#[derive(Debug, thiserror::Error)]
pub enum MangadexError {
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error("cannot parse chapter id from '{0}'")]
    UrlParseError(String),
    #[error("cannot deserialize the response")]
    DeserializeError,
    #[error("cannot get manga title")]
    CannotGetManga,
}

impl MangadexChapter {
    pub async fn from_url(url: impl IntoUrl) -> Result<Self, MangadexError> {
        let url = url.into_url()?;
        let mut segments = url
            .path_segments()
            .ok_or_else(|| MangadexError::UrlParseError(url.to_string()))?;
        if segments.next() != Some("chapter") {
            return Err(MangadexError::UrlParseError(url.to_string()));
        }
        let chapter_id = segments
            .next()
            .ok_or_else(|| MangadexError::UrlParseError(url.to_string()))?;

        let (manga_title, chapter_title, volume, chapter) = get_chapter_info(chapter_id).await?;
        let pages = get_chapter_pages(chapter_id).await?;

        Ok(Self {
            url: url.to_string(),
            manga_title,
            chapter_title,
            volume,
            chapter,
            pages,
        })
    }
}

async fn get_chapter_info(
    chapter_id: &str,
) -> Result<(String, Option<String>, Option<String>, Option<String>), MangadexError> {
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ResponseBody {
        data: ChapterData,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ChapterData {
        attributes: ChapterAttributes,
        relationships: Vec<Relationship>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Relationship {
        // id: String,
        #[serde(rename = "type")]
        relationship_type: String,
        attributes: Option<RelationshipAttributes>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RelationshipAttributes {
        title: HashMap<String, String>,
        // alt_titles: Vec<HashMap<String, String>>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ChapterAttributes {
        title: Option<String>,
        volume: Option<String>,
        chapter: Option<String>,
    }

    let response = reqwest::get(&format!(
        "https://api.mangadex.org/chapter/{chapter_id}?includes[]=manga"
    ))
    .await?
    .error_for_status()?;
    let json = response.text().await?;
    let chapter_info: ResponseBody = serde_json::from_str(&json).map_err(|e| {
        error!("Cannot deserialize {}. Error: {}", json, e);
        MangadexError::DeserializeError
    })?;

    let manga_title = chapter_info
        .data
        .relationships
        .iter()
        .find(|x| x.relationship_type == "manga")
        .and_then(|x| x.attributes.as_ref())
        .and_then(|attr| attr.title.values().next().map(|x| x.to_string()))
        .ok_or(MangadexError::CannotGetManga)?;

    Ok((
        manga_title,
        chapter_info.data.attributes.title,
        chapter_info.data.attributes.volume,
        chapter_info.data.attributes.chapter,
    ))
}

async fn get_chapter_pages(chapter_id: &str) -> Result<Vec<DownloadItem>, MangadexError> {
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ResponseBody {
        base_url: String,
        chapter: ChapterData,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ChapterData {
        hash: String,
        data_saver: Vec<String>,
    }

    let response = reqwest::get(format!(
        "https://api.mangadex.org/at-home/server/{chapter_id}"
    ))
    .await?
    .error_for_status()?;
    let json = response.text().await?;
    let chapter_json: ResponseBody = serde_json::from_str(&json).map_err(|e| {
        error!("Cannot deserialize {}. Error: {}", json, e);
        MangadexError::DeserializeError
    })?;
    let pages: Vec<_> = chapter_json
        .chapter
        .data_saver
        .iter()
        .enumerate()
        .map(|(index, page_hash)| {
            DownloadItem::new(
                &format!(
                    "{}/data-saver/{}/{}",
                    chapter_json.base_url, chapter_json.chapter.hash, page_hash
                ),
                Some(&format!("page_{:03}", index + 1)),
            )
        })
        .collect();
    Ok(pages)
}

impl Chapter for MangadexChapter {
    fn url(&self) -> String {
        self.url.clone()
    }

    fn manga(&self) -> String {
        self.manga_title.clone()
    }

    fn chapter(&self) -> String {
        let chapter = self.chapter.clone().unwrap_or(String::from("0"));
        match (self.volume.as_ref(), self.chapter_title.as_ref()) {
            (Some(v), Some(t)) => format!("vol {v} chap {chapter} - {t}"),
            (Some(v), None) => format!("vol {v} chap {chapter}"),
            (None, Some(t)) => format!("chap {chapter} - {t}"),
            (None, None) => format!("chap {chapter}"),
        }
    }

    fn pages_download_info(&self) -> &Vec<DownloadItem> {
        &self.pages
    }
}
