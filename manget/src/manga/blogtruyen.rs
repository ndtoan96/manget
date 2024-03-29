use reqwest::IntoUrl;
use scraper::{Html, Selector};

use crate::{download::DownloadItem, manga::Chapter};

#[derive(Debug, thiserror::Error)]
pub enum BlogTruyenError {
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    ParseError(&'static str),
}

#[derive(Debug)]
pub struct BlogTruyenChapter {
    url: String,
    manga: String,
    chapter: String,
    pages: Vec<DownloadItem>,
    referer: String,
}

impl BlogTruyenChapter {
    pub async fn from_url(url: impl IntoUrl + Clone + ToString) -> Result<Self, BlogTruyenError> {
        let mut url = url.into_url().unwrap();
        if url.domain().is_some_and(|x| x.starts_with("m.")) {
            url.set_host(Some("blogtruyenmoi.com")).unwrap();
        }
        let response = reqwest::Client::new()
            .get(url.clone())
            .header("Accept", "*/*")
            .header("User-Agent", "Manget")
            .send()
            .await?
            .error_for_status()?;
        // let response = reqwest::get(url.clone()).await?.error_for_status()?;
        let html_content = response.text().await?;

        let html = Html::parse_document(&html_content);
        let title_selector = Selector::parse("header > div.breadcrumbs").unwrap();

        let title_elem = html
            .select(&title_selector)
            .next()
            .ok_or(BlogTruyenError::ParseError("cannot find title"))?;
        let mut text_iter = title_elem.text();
        text_iter.next(); // to ignore newline
        text_iter.next();
        text_iter.next();
        let manga = text_iter.next().unwrap_or("").trim().to_string();
        let chapter = text_iter
            .next()
            .unwrap_or("")
            .trim()
            .trim_start_matches("> ")
            .replacen(&manga, "", 1)
            .trim()
            .to_string();

        let img_selector = Selector::parse("article#content > img").unwrap();
        let mut pages = Vec::new();
        for (i, img_elem) in html.select(&img_selector).enumerate() {
            let src = img_elem.value().attr("src").unwrap();
            let ext = if src.contains(".png") { "png" } else { "jpg" };
            pages.push(DownloadItem::new(
                src,
                Some(&format!("page_{:02}.{}", i, ext)),
            ));
        }
        let referer = format!("https://{}/", url.domain().unwrap_or_default());
        Ok(Self {
            url: url.to_string(),
            manga,
            chapter,
            pages,
            referer,
        })
    }
}

impl Chapter for BlogTruyenChapter {
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
        Some(self.referer.clone())
    }
}

#[cfg(test)]
#[tokio::test]
async fn test_build_blogtruyen_chapter() {
    let chapter = BlogTruyenChapter::from_url(
        "https://blogtruyen.vn/c656991/nise-koi-chap-2295-ngoai-truyen",
    )
    .await
    .unwrap();
    dbg!(&chapter);
    assert_eq!(chapter.manga.to_lowercase(), "nisekoi");
    assert!(chapter.chapter.to_lowercase().contains("ngoại truyện"));
    assert!(!chapter.pages_download_info().is_empty());
}

#[cfg(test)]
#[tokio::test]
async fn test_build_blogtruyenmoi_chapter() {
    let chapter = BlogTruyenChapter::from_url(
        "https://blogtruyenmoi.com/c809137/kuroiwa-medaka-ni-watasgu-no-kawaii-ga-tsuujinai-chap-95-co-gai-do-va-nhung-gioi-luat",
    )
    .await
    .unwrap();
    dbg!(&chapter);
    assert!(chapter.manga.to_lowercase().contains("kuroiwa"));
    assert!(chapter.chapter.to_lowercase().contains("95"));
    assert!(!chapter.pages_download_info().is_empty());
}

#[cfg(test)]
#[tokio::test]
async fn test_build_blogtruyen_mobile_chapter() {
    let chapter = BlogTruyenChapter::from_url(
        "https://m.blogtruyenmoi.com/c824113/the-duke-of-death-and-his-black-maid-chuong-168-loi-hua",
    )
    .await
    .unwrap();
    dbg!(&chapter);
    assert!(chapter.manga.to_lowercase().contains("công tước"));
    assert!(chapter.chapter.contains("168"));
    assert!(!chapter.pages_download_info().is_empty());
}
