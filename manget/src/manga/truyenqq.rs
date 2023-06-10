use reqwest::IntoUrl;
use scraper::{Html, Selector};

use crate::{download::DownloadItem, manga::Chapter};

#[derive(Debug, thiserror::Error)]
pub enum TruyenqqError {
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    ParseError(&'static str),
}

#[derive(Debug)]
pub struct TruyenqqChapter {
    url: String,
    manga: String,
    chapter: String,
    pages: Vec<DownloadItem>,
}

impl TruyenqqChapter {
    pub async fn from_url(url: impl IntoUrl + Clone + ToString) -> Result<Self, TruyenqqError> {
        let response = reqwest::get(url.clone()).await?.error_for_status()?;
        let html_content = response.text().await?;

        let html = Html::parse_document(&html_content);
        let title_selector = Selector::parse("h1.detail-title").unwrap();

        let h1_elm = html
            .select(&title_selector)
            .next()
            .ok_or(TruyenqqError::ParseError("cannot find title"))?;
        let mut text_iter = h1_elm.text();
        text_iter.next(); // to ignore newline
        let manga = text_iter.next().unwrap_or("").trim().to_string();
        let chapter = text_iter
            .next()
            .unwrap_or("")
            .trim()
            .trim_start_matches("- ")
            .to_string();

        let img_selector = Selector::parse("img.lazy[referrerpolicy=\"origin\"]").unwrap();
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

impl Chapter for TruyenqqChapter {
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
        Some("https://truyenqq.com.vn/".to_string())
    }
}

#[cfg(test)]
#[tokio::test]
async fn test_build_truyenqq_chapter() {
    let chapter = TruyenqqChapter::from_url(
        "https://truyenqq.com.vn/truyen-tranh/grand-blue-co-gai-thich-lan/chuong-85/749049",
    )
    .await
    .unwrap();
    dbg!(chapter);
}
