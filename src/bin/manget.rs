use std::path::PathBuf;

use clap::Parser;
use log::LevelFilter;
use manget::manga::{
    download_chapter, download_chapter_as_cbz, generate_chapter_full_name, get_chapter,
};
use reqwest::Url;
use simple_logger::SimpleLogger;

/// Manga download tool
#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    out_dir: Option<PathBuf>,
    #[arg(long)]
    cbz: bool,
    url: Url,
}

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let args = Args::parse();

    match get_chapter(args.url.clone()).await {
        Some(chapter) => {
            if args.cbz {
                if let Err(e) = download_chapter_as_cbz(
                    &chapter,
                    args.out_dir.map(|p| {
                        p.join(generate_chapter_full_name(&chapter))
                            .with_extension("cbz")
                    }),
                )
                .await
                {
                    eprintln!("{e}");
                }
            } else if let Err(e) = download_chapter(
                &chapter,
                args.out_dir
                    .map(|p| p.join(generate_chapter_full_name(&chapter))),
            )
            .await
            {
                eprintln!("{e}");
            }
        }
        None => eprintln!("Cannot get chapter info from {}", args.url),
    }
}
