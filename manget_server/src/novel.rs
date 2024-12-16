use scraper::Selector;

pub fn convert_chapter_html_to_epub(title: &str, content: &str) -> epub_builder::Result<Vec<u8>> {
    let processed_content = process_chapter_content(content);
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
    epub_builder::EpubBuilder::new(epub_builder::ZipLibrary::new()?)?
        .metadata("title", title)?
        .epub_version(epub_builder::EpubVersion::V30)
        .add_content(
            epub_builder::EpubContent::new("chapter.xhtml", xhtml.as_bytes())
                .title(title)
                .reftype(epub_builder::ReferenceType::Text),
        )?
        .generate(&mut output)?;
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
