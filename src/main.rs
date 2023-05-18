use std::time::Duration;
use regex::Regex;
use serde::Deserialize;
use mget::Downloader;

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
struct ChapterInfo {
    http_lis: Vec<String>,
    word_lis: Vec<String>,
}

#[tokio::main]
async fn main() {
    // Downloader::new()
    //     .limit_speed(1, Duration::from_secs(1))
    //     .set_path("./downloads")
    //     .unwrap()
    //     .add_url("http://truyentuan.com/wp-content/banners/neo3T.png")
    //     .add_url("http://truyentuan.com/manga2/one-piece/1084/tcbop_1084_001.jpg")
    //     .add_url("http://truyentuan.com/manga2/one-piece/1084/tcbop_1084_002.jpg")
    //     .download()
    //     .await;

    let response = reqwest::get("https://mangapark.net/title/74968-mato-seihei-no-slave/7968180-en-vol.13-ch.106").await.unwrap();
    let body = response.text().await.unwrap();
    let pattern = Regex::new(r#"\{"httpLis".*?\}"#).unwrap();
    let find_result = pattern.find(&body).unwrap();
    let chapter_info = serde_json::from_str::<ChapterInfo>(find_result.as_str()).unwrap();
    let mut downloader = Downloader::new().set_path("./downloads").unwrap();
    for (index, (url, params)) in chapter_info.http_lis.iter().zip(chapter_info.word_lis.iter()).enumerate() {
        let complete_url = format!("{url}?{params}");
        downloader.add_url_with_name(&complete_url, &format!("page_{index:02}"));
    }
    downloader.download().await;
}
