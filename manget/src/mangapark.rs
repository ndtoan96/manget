use regex::Regex;
use reqwest::IntoUrl;
use serde::Deserialize;

use crate::{download::DownloadItem, manga::Chapter};

type Result<T> = std::result::Result<T, MangaParkError>;

#[derive(Debug, thiserror::Error)]
pub enum MangaParkError {
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),
    #[error("cannot find chapter download info")]
    ParseError,
}

pub struct MangaParkChapter {
    url: String,
    title: String,
    chapter: Option<String>,
    pages: Vec<DownloadItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChapterDownloadInfo {
    http_lis: Vec<String>,
    word_lis: Vec<String>,
}

impl MangaParkChapter {
    pub async fn from_url(url: impl IntoUrl) -> Result<Self> {
        let url = url.into_url()?;
        let html = reqwest::get(url.clone())
            .await?
            .error_for_status()?
            .text()
            .await?;
        let download_items = get_chapter_download_info(&html)?;
        let (title, chapter) = get_title_and_chapter_name(&html);
        match title {
            Some(t) => Ok(Self {
                url: url.as_str().to_string(),
                title: t,
                chapter,
                pages: download_items,
            }),
            None => Err(MangaParkError::ParseError),
        }
    }
}

impl Chapter for MangaParkChapter {
    fn url(&self) -> String {
        self.url.to_string()
    }

    fn title(&self) -> String {
        self.title.to_string()
    }

    fn chapter_name(&self) -> String {
        self.chapter.as_deref().unwrap_or("chapter 0").to_string()
    }

    fn pages_download_info(&self) -> &Vec<DownloadItem> {
        &self.pages
    }
}

fn get_title_and_chapter_name(html: &str) -> (Option<String>, Option<String>) {
    let pattern = Regex::new(
        r"<title>(?P<title>.*) - (?P<chapter>.*) - Share Any Manga at MangaPark</title>",
    )
    .unwrap();
    match pattern.captures(html) {
        None => (None, None),
        Some(cap) => (
            cap.name("title")
                .map(|x| x.as_str())
                .map(|x| html_escape::decode_html_entities(x).to_string()),
            cap.name("chapter")
                .map(|x| x.as_str())
                .map(|x| html_escape::decode_html_entities(x).to_string()),
        ),
    }
}

fn get_chapter_download_info(html: &str) -> Result<Vec<DownloadItem>> {
    let pattern = Regex::new(r#"\{"httpLis".*?\}"#).unwrap();
    let download_info_raw = pattern
        .find(html)
        .ok_or(MangaParkError::ParseError)?
        .as_str();
    let download_info: ChapterDownloadInfo =
        serde_json::from_str(download_info_raw).map_err(|_| MangaParkError::ParseError)?;
    let mut download_items = Vec::new();
    for (index, (url, params)) in download_info
        .http_lis
        .iter()
        .zip(download_info.word_lis.iter())
        .enumerate()
    {
        let complete_url = format!("{url}?{params}");
        download_items.push(DownloadItem::new(
            &complete_url,
            Some(&format!("page_{index:03}")),
        ));
    }
    Ok(download_items)
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_get_title_volume_chapter() {
        let html = reqwest::get(
            "https://mangapark.net/title/74968-mato-seihei-no-slave/7968180-en-vol.13-ch.106",
        )
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
        assert_eq!(
            get_title_and_chapter_name(&html),
            (
                Some(String::from("Mato Seihei no Slave")),
                Some(String::from("Vol.13 Ch.106")),
            )
        );

        let html = reqwest::get(
            "https://mangapark.net/title/97490-koi-shita-no-de-haishin-shite-mita/2686868-en-ch.57",
        )
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
        assert_eq!(
            get_title_and_chapter_name(&html),
            (
                Some(String::from("Koi Shita no de, Haishin Shite Mita")),
                Some(String::from("Ch.057")),
            )
        );
    }
}
