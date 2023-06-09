use std::{error::Error, fs, path::PathBuf};

use clap::Parser;
use manget::manga::{
    download_chapter, download_chapter_as_cbz, generate_chapter_full_name, get_chapter,
};

/// Manga download tool
#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    out_dir: Option<PathBuf>,
    #[arg(long)]
    cbz: bool,
    #[arg(short, long, group = "group_file", conflicts_with = "group_url")]
    file: Option<PathBuf>,
    #[arg(group = "group_url")]
    url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    match (args.url, args.file) {
        (Some(url), _) => {
            env_logger::init();
            download_one(&url, &args.out_dir, args.cbz).await?;
        }
        (_, Some(file)) => {
            let content = fs::read_to_string(&file)?;
            let all_result = futures::future::join_all(
                content
                    .lines()
                    .map(|url| download_one(url, &args.out_dir, args.cbz)),
            )
            .await;
            if let Some(r) = all_result.into_iter().find(|r| r.is_err()) {
                return r;
            }
        }
        (None, None) => unreachable!(),
    }

    Ok(())
}

async fn download_one(
    url: &str,
    out_dir: &Option<PathBuf>,
    cbz: bool,
) -> Result<(), Box<dyn Error>> {
    let chapter = get_chapter(url).await?;
    if cbz {
        download_chapter_as_cbz(
            &chapter,
            out_dir.as_ref().map(|p| {
                p.join(generate_chapter_full_name(&chapter))
                    .with_extension("cbz")
            }),
        )
        .await?;
    } else {
        download_chapter(
            &chapter,
            out_dir
                .as_ref()
                .map(|p| p.join(generate_chapter_full_name(&chapter))),
        )
        .await?;
    }

    println!("Downloaded: '{}'", generate_chapter_full_name(&chapter));

    Ok(())
}
