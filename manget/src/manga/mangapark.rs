use regex::Regex;
use reqwest::IntoUrl;
use scraper::{Html, Selector};

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
    manga_title: String,
    chapter: Option<String>,
    pages: Vec<DownloadItem>,
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
        let (title, chapter) = get_title_and_chapter_name(&html)?;
        Ok(Self {
            url: url.as_str().to_string(),
            manga_title: title,
            chapter: Some(chapter),
            pages: download_items,
        })
    }
}

impl Chapter for MangaParkChapter {
    fn url(&self) -> String {
        self.url.to_string()
    }

    fn manga(&self) -> String {
        self.manga_title.to_string()
    }

    fn chapter(&self) -> String {
        self.chapter.as_deref().unwrap_or("chapter 0").to_string()
    }

    fn pages_download_info(&self) -> &Vec<DownloadItem> {
        &self.pages
    }
}

fn get_title_and_chapter_name(html: &str) -> Result<(String, String)> {
    let doc = Html::parse_document(html);
    let title_selector = Selector::parse("h3 > a[href^=\"/title\"]").unwrap();
    let chapter_selector = Selector::parse("h6 > a[href^=\"/title\"]").unwrap();
    let title = doc
        .select(&title_selector)
        .next()
        .ok_or(MangaParkError::ParseError)?
        .text()
        .collect::<Vec<&str>>()
        .join("");
    let chapter = doc
        .select(&chapter_selector)
        .next()
        .ok_or(MangaParkError::ParseError)?
        .text()
        .collect::<Vec<&str>>()
        .join("");
    Ok((title, chapter))
}

fn get_chapter_download_info(html: &str) -> Result<Vec<DownloadItem>> {
    let pattern = Regex::new(r#""/title/[^"]+",(?:"https://[^"]+",)+"#).unwrap();
    let captured = pattern
        .captures(html)
        .ok_or(MangaParkError::ParseError)?
        .get(0)
        .ok_or(MangaParkError::ParseError)?
        .as_str();
    let download_items = captured
        .split(',')
        .skip(1)
        .take_while(|s| !s.is_empty())
        .map(|s| s.trim_start_matches('"').trim_end_matches('"'))
        .enumerate()
        .map(|(i, url)| DownloadItem::new(url, Some(format!("page_{:03}", i))))
        .collect();
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
            get_title_and_chapter_name(&html).unwrap(),
            (
                String::from("Mato Seihei no Slave"),
                String::from("Vol.13 Ch.106: Bell's Tears"),
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
            get_title_and_chapter_name(&html).unwrap(),
            (
                String::from("Koi Shita no de, Haishin Shite Mita"),
                String::from("Ch.057"),
            )
        );
    }

    #[tokio::test]
    async fn test_get_download_info() {
        let html = reqwest::get(
            "https://mangapark.net/title/74968-mato-seihei-no-slave/7968180-en-vol.13-ch.106",
        )
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

        let download_info = get_chapter_download_info(&html).unwrap();
        dbg!(&download_info);
        assert!(download_info.len() > 10);
    }
}
