use std::io::Cursor;

use reqwest::Url;
use scraper::{Html, Selector};

struct Image {
    url: String,
    mime_type: String,
    name: String,
    data: Vec<u8>,
}

pub async fn convert_chapter_html_to_epub(
    title: &str,
    content: &str,
) -> epub_builder::Result<Vec<u8>> {
    let mut processed_content = process_chapter_content(content);
    let images = extract_images(&processed_content).await;

    for image in &images {
        processed_content =
            processed_content.replace(&image.url, &format!("Images/{}", image.name));
    }

    let xhtml = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html>

<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head>
  <title>{title}</title>
</head>

<body>
{processed_content}
</body>
</html>
"#
    );

    let mut output = Vec::new();
    let mut builder = epub_builder::EpubBuilder::new(epub_builder::ZipLibrary::new()?)?;
    builder
        .metadata("title", title)?
        .epub_version(epub_builder::EpubVersion::V30)
        .add_content(
            epub_builder::EpubContent::new("chapter.xhtml", xhtml.as_bytes())
                .title(title)
                .reftype(epub_builder::ReferenceType::Text),
        )?;

    for image in images {
        builder.add_resource(
            format!("Images/{}", image.name),
            Cursor::new(image.data),
            image.mime_type,
        )?;
    }

    builder.generate(&mut output)?;
    Ok(output)
}

fn process_chapter_content(content: &str) -> String {
    let html = scraper::Html::parse_fragment(content);
    let selector = Selector::parse(".br-section > *").unwrap();
    let texts: Vec<_> = html
        .select(&selector)
        .filter(|e| e.value().name() != "div")
        .map(|e| e.html())
        .map(|t| {
            if t.starts_with("<img") {
                t.replace(">", "/>")
            } else {
                t
            }
        })
        .collect();
    texts
        .join("\n")
        .replace("<br>", "<br/>")
        .replace("<hr>", "<hr/>")
}

async fn extract_images(content: &str) -> Vec<Image> {
    let urls = {
        let html = Html::parse_document(content);
        let selector = Selector::parse("img").unwrap();
        let img_elements = html.select(&selector);
        img_elements
            .into_iter()
            .filter_map(|img| img.value().attr("src"))
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    };
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    for url in urls {
        let thread_tx = tx.clone();
        tokio::spawn(async move {
            tracing::info!("Download {}", &url);
            let result = reqwest::get(&url).await;
            thread_tx.send((url, result)).unwrap();
        });
    }
    drop(tx);
    let mut images = Vec::new();
    while let Some((url, result)) = rx.recv().await {
        tracing::info!("Recieve url {}", &url);
        if let Ok(res) = result.and_then(|res| res.error_for_status()) {
            let mime_type = res
                .headers()
                .get("content-type")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            let data = res.bytes().await.unwrap().to_vec();
            let name = Url::parse(&url)
                .unwrap()
                .path_segments()
                .unwrap()
                .last()
                .unwrap()
                .to_string();
            images.push(Image {
                url,
                mime_type,
                data,
                name,
            });
        }
    }
    tracing::info!("DONE");
    images
}
