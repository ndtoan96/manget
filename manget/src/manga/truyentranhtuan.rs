use std::path::Path;

use regex::RegexBuilder;
use reqwest::IntoUrl;
use scraper::{Html, Selector};

use crate::{download::DownloadItem, manga::Chapter};

#[derive(Debug, thiserror::Error)]
pub enum TruyenTranhTuanError {
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),
    #[error(transparent)]
    RegexError(#[from] regex::Error),
    #[error("Parse error: {0}")]
    ParseError(&'static str),
    #[error(transparent)]
    CannotDeserialize(#[from] serde_json::Error),
}

#[derive(Debug)]
pub struct TruyenTranhTuanChapter {
    url: String,
    manga: String,
    chapter: String,
    pages: Vec<DownloadItem>,
}

impl TruyenTranhTuanChapter {
    pub async fn from_url(
        url: impl IntoUrl + Clone + ToString,
    ) -> Result<Self, TruyenTranhTuanError> {
        let response = reqwest::get(url.clone()).await?.error_for_status()?;
        let html_content = response.text().await?;

        let html = Html::parse_document(&html_content);
        let title_selector = Selector::parse("div#read-title").unwrap();

        let h1_elm = html
            .select(&title_selector)
            .next()
            .ok_or(TruyenTranhTuanError::ParseError("cannot find title"))?;
        let mut text_iter = h1_elm.text();
        text_iter.next(); // to ignore newline
        text_iter.next(); // to ignore newline
        let manga = text_iter.next().unwrap_or("").trim().to_string();
        let chapter = text_iter
            .next()
            .unwrap_or("")
            .trim()
            .trim_start_matches("> ")
            .to_string();

        let mut pages = Vec::new();
        let url_list_str = RegexBuilder::new(r#"slides_page_path = (\[.*?\])"#)
            .multi_line(true)
            .dot_matches_new_line(true)
            .build()?
            .captures(&html_content)
            .ok_or(TruyenTranhTuanError::ParseError("cannot find chapter list"))?
            .get(1)
            .ok_or(TruyenTranhTuanError::ParseError(
                "cannot parse chapter list",
            ))?
            .as_str();
        let url_list: Vec<String> = serde_json::from_str(url_list_str)?;
        for page_url in url_list {
            let file_name = Path::new(&page_url)
                .file_name()
                .map(|x| x.to_string_lossy().into_owned());
            pages.push(DownloadItem::new(&page_url, file_name.as_deref()));
        }
        Ok(Self {
            url: url.to_string(),
            manga,
            chapter,
            pages,
        })
    }
}

impl Chapter for TruyenTranhTuanChapter {
    fn url(&self) -> String {
        self.url.to_string()
    }

    fn manga(&self) -> String {
        self.manga.clone()
    }

    fn chapter(&self) -> String {
        self.chapter.clone()
    }

    fn pages_download_info(&self) -> &Vec<DownloadItem> {
        &self.pages
    }
}

#[cfg(test)]
#[tokio::test]
async fn test_build_truyentranhtuan_chapter() {
    let chapter = TruyenTranhTuanChapter::from_url("http://truyentuan.com/one-piece-chuong-1086/")
        .await
        .unwrap();
    dbg!(&chapter);
    assert!(chapter.manga.to_lowercase() == "one piece");
    assert!(chapter.chapter.contains("1086"));
    assert!(!chapter.pages.is_empty());
}
