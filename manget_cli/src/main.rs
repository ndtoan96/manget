use std::{
    fs,
    io::{Read, Write},
    ops::Deref,
    path::{Path, PathBuf},
    time::Duration,
};

use clap::{Args, Parser};
use manget::manga::{download_chapter, download_chapter_as_cbz, get_chapter, ChapterError};
use tower::{
    limit::{ConcurrencyLimitLayer, RateLimitLayer},
    Service, ServiceBuilder, ServiceExt,
};
use zip::{write::FileOptions, ZipWriter};

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
    #[arg(long = "rev", help = "reverse order of input urls")]
    reverse: bool,
    #[arg(long = "make-cbz", help = "make a cbz file")]
    make_cbz: bool,
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

            let urls: Box<dyn Iterator<Item = &str>> = if args.batch_args.reverse {
                Box::new(content.trim().lines().rev())
            } else {
                Box::new(content.trim().lines())
            };

            let mut downloaded_paths = Vec::new();

            for url in urls {
                let request = DownloadRequest {
                    url: url.to_string(),
                    out_dir: args.out_dir.clone(),
                    cbz: args.cbz,
                };
                match download_service.ready().await?.call(request).await {
                    Err(e) => {
                        if !args.batch_args.ignore_error {
                            return Err(e);
                        } else {
                            eprintln!("{e}");
                        }
                    }
                    Ok(path) => downloaded_paths.push(path),
                }
            }

            if args.batch_args.make_cbz {
                println!("Making cbz...");
                make_cbz(&downloaded_paths)?;
                println!("Done.");
            }
        }
        (None, None) => unreachable!(),
    }

    Ok(())
}

async fn download_one(request: DownloadRequest) -> Result<PathBuf, ChapterError> {
    let url = request.url;
    let out_dir = request.out_dir;
    let cbz = request.cbz;

    let chapter_own = get_chapter(url).await?;
    let chapter = chapter_own.deref();
    let downloaded_path = if cbz {
        download_chapter_as_cbz(
            chapter,
            out_dir
                .as_ref()
                .map(|p| p.join(chapter.full_name()).with_extension("cbz")),
        )
        .await?
    } else {
        download_chapter(
            chapter,
            out_dir.as_ref().map(|p| p.join(chapter.full_name())),
        )
        .await?
    };

    println!(
        "Downloaded: '{}'",
        downloaded_path.file_name().unwrap().to_string_lossy()
    );

    Ok(downloaded_path)
}

fn make_cbz<T1, T2>(paths: T1) -> Result<(), std::io::Error>
where
    T1: IntoIterator<Item = T2>,
    T2: AsRef<Path>,
{
    let mut new_names = Vec::new();
    let mut parent = None;
    for (i, path) in paths.into_iter().enumerate() {
        let path = path.as_ref();
        parent = Some(path.parent().unwrap_or(Path::new(".")).to_path_buf());
        let current_name = path.file_name().unwrap();
        let new_name = format!("{:05}_{}", i, current_name.to_string_lossy());
        let new_path = path.with_file_name(&new_name);
        fs::rename(path, &new_path)?;
        new_names.push(new_name);
    }

    if new_names.is_empty() {
        return Ok(());
    }

    let parent = parent.unwrap();

    // zip all folder and create cbz file
    let file = fs::File::create(parent.join("manga.cbz"))?;
    let mut writer = ZipWriter::new(file);
    let mut buf = Vec::new();
    for name in new_names.iter() {
        // writer.add_directory(name, FileOptions::default())?;
        for entry in fs::read_dir(parent.join(name))? {
            let file_path = entry?.path();
            if file_path.is_file() {
                writer.start_file(
                    format!(
                        "{}/{}",
                        name,
                        file_path.file_name().unwrap().to_string_lossy()
                    ),
                    FileOptions::default(),
                )?;

                fs::File::open(file_path)?.read_to_end(&mut buf)?;
                writer.write_all(&buf)?;
                buf.clear();
            }
        }
        // The folder has been added to cbz, delete it
        let _ = fs::remove_dir_all(parent.join(name));
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::path::{Path, PathBuf};

    use crate::{download_one, DownloadRequest};

    struct TestResource {
        dir: PathBuf,
    }

    impl TestResource {
        fn new(path: impl AsRef<Path>) -> Self {
            Self {
                dir: path.as_ref().to_path_buf(),
            }
        }
    }

    impl Drop for TestResource {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[tokio::test]
    async fn test_download_one() {
        let resource = TestResource::new("test");
        let download_request = DownloadRequest {
            url: "https://mangadex.org/chapter/f9a8fc1f-1fb5-43af-8844-1672ee6c7290".to_string(),
            cbz: false,
            out_dir: Some(resource.dir.clone()),
        };
        download_one(download_request).await.unwrap();
    }
}
