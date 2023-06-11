use reqwest::IntoUrl;
use scraper::{Html, Selector};

use crate::{download::DownloadItem, manga::Chapter};

#[derive(Debug, thiserror::Error)]
pub enum TopTruyenError {
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    ParseError(&'static str),
}

#[derive(Debug)]
pub struct TopTruyenChapter {
    url: String,
    manga: String,
    chapter: String,
    pages: Vec<DownloadItem>,
}

impl TopTruyenChapter {
    pub async fn from_url(url: impl IntoUrl + Clone + ToString) -> Result<Self, TopTruyenError> {
        let response = reqwest::get(url.clone()).await?.error_for_status()?;
        let html_content = response.text().await?;

        let html = Html::parse_document(&html_content);
        let title_selector = Selector::parse("h1.chapter-info").unwrap();

        let h1_elm = html
            .select(&title_selector)
            .next()
            .ok_or(TopTruyenError::ParseError("cannot find title"))?;
        let mut text_iter = h1_elm.text();
        text_iter.next(); // to ignore newline
        let manga = text_iter.next().unwrap_or("").trim().to_string();
        text_iter.next(); // ignore newline
        let chapter = text_iter
            .next()
            .unwrap_or("")
            .trim()
            .trim_start_matches("- ")
            .to_string();

        let img_selector = Selector::parse("div.page-chapter[id^=\"page\"] > img").unwrap();
        let mut pages = Vec::new();
        for (i, img_elem) in html.select(&img_selector).enumerate() {
            let src = img_elem.value().attr("src").unwrap();
            let ext = if src.contains(".png") { "png" } else { "jpg" };
            pages.push(DownloadItem::new(
                src,
                Some(&format!("page_{:02}.{}", i, ext)),
            ));
        }
        Ok(Self {
            url: url.to_string(),
            manga,
            chapter,
            pages,
        })
    }
}

impl Chapter for TopTruyenChapter {
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

    fn referer(&self) -> Option<String> {
        Some("https://www.toptruyen.live/".to_string())
    }
}

#[cfg(test)]
#[tokio::test]
async fn test_build_toptruyen_chapter() {
    let chapter = TopTruyenChapter::from_url(
        "https://www.toptruyen.live/truyen-tranh/grand-blue-co-gai-thich-lan/chapter-81/771033",
    )
    .await
    .unwrap();
    dbg!(chapter);
}