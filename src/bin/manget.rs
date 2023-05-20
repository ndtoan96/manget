use log::LevelFilter;
use manget::{manga::download_chapter, mangapark};
use simple_logger::SimpleLogger;

#[tokio::main]
async fn main() {
    SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();
    match mangapark::MangaParkChapter::from("https://mangapark.net/title/74602-edens-zero/7974708-en-ch.238").await {
        Ok(chapter) => if let Err(e) = download_chapter(&chapter, None).await {
            eprintln!("{e}");
        },
        Err(e) => eprintln!("{e}"),
    }
    
}