use std::{fs, path::PathBuf, time::Duration};

use clap::{Args, Parser};
use manget::manga::{
    download_chapter, download_chapter_as_cbz, generate_chapter_full_name, get_chapter,
    ChapterError,
};
use tower::{
    limit::{ConcurrencyLimitLayer, RateLimitLayer},
    Service, ServiceBuilder, ServiceExt,
};

/// Manga download tool
#[derive(Debug, Parser)]
#[command(author, version, about)]
struct DownloadArgs {
    /* Common */
    #[arg(short, long)]
    out_dir: Option<PathBuf>,
    #[arg(long)]
    cbz: bool,

    /* Group URL */
    #[arg(conflicts_with = "group_batch")]
    url: Option<String>,

    #[command(flatten)]
    batch_args: BatchDownloadArgs,
}

#[derive(Debug, Args)]
#[group(id = "group_batch")]
struct BatchDownloadArgs {
    #[arg(short, long)]
    file: Option<PathBuf>,
    #[arg(
        long = "continue",
        help = "continue to download even if there is error"
    )]
    ignore_error: bool,
    #[arg(long = "cl", help = "concurrency limt")]
    concurrency_limit: Option<usize>,
    #[arg(long = "max-chap", help = "set rate limit, used along with --per")]
    max_chap: Option<u64>,
    #[arg(
        long = "per-secs",
        help = "set rate limit (seconds), used along with --max-chap"
    )]
    duration: Option<u64>,
}

struct DownloadRequest {
    url: String,
    out_dir: Option<PathBuf>,
    cbz: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = DownloadArgs::parse();
    env_logger::init();

    match (args.url, args.batch_args.file) {
        (Some(url), _) => {
            download_one(DownloadRequest {
                url: url.to_string(),
                out_dir: args.out_dir.clone(),
                cbz: args.cbz,
            })
            .await?;
        }
        (_, Some(file)) => {
            let content = fs::read_to_string(&file)?;

            let maybe_concurrency_limit = args
                .batch_args
                .concurrency_limit
                .map(ConcurrencyLimitLayer::new);

            let maybe_rate_limit = if let (Some(max_chap), Some(dur)) =
                (args.batch_args.max_chap, args.batch_args.duration)
            {
                Some(RateLimitLayer::new(max_chap, Duration::from_secs(dur)))
            } else {
                None
            };

            // Create a download service
            let mut download_service = ServiceBuilder::new()
                .option_layer(maybe_concurrency_limit)
                .option_layer(maybe_rate_limit)
                .service_fn(download_one);

            for url in content.lines() {
                let request = DownloadRequest {
                    url: url.to_string(),
                    out_dir: args.out_dir.clone(),
                    cbz: args.cbz,
                };
                if let Err(e) = download_service.ready().await?.call(request).await {
                    if !args.batch_args.ignore_error {
                        return Err(e);
                    } else {
                        eprintln!("{e}");
                    }
                }
            }
        }
        (None, None) => unreachable!(),
    }

    Ok(())
}

async fn download_one(request: DownloadRequest) -> Result<(), ChapterError> {
    let url = request.url;
    let out_dir = request.out_dir;
    let cbz = request.cbz;

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
