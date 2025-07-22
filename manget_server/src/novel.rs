use std::io::Cursor;

use image::ImageReader;
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
            let result = reqwest::get(&url).await;
            thread_tx.send((url, result)).unwrap();
        });
    }
    drop(tx);
    let mut images = Vec::new();
    while let Some((url, result)) = rx.recv().await {
        if let Ok(res) = result.and_then(|res| res.error_for_status()) {
            let tmp_data = res.bytes().await.unwrap().to_vec();
            let img = ImageReader::new(Cursor::new(tmp_data))
                .with_guessed_format()
                .unwrap()
                .decode()
                .unwrap();
            let mut data = Vec::new();
            img.write_to(&mut Cursor::new(&mut data), image::ImageFormat::Jpeg)
                .unwrap();
            let name = Url::parse(&url)
                .unwrap()
                .path_segments()
                .unwrap()
                .next_back()
                .unwrap()
                .to_string();
            images.push(Image {
                url,
                mime_type: "image/jpeg".to_string(),
                data,
                name,
            });
        }
    }
    images
}
