use std::io::{Cursor, Read, Write};
use thiserror::Error;
use word_core::{Block, Document, Inline, Paragraph, Section, StyleId};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const ODT_MIME_TYPE: &str = "application/vnd.oasis.opendocument.text";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackageLimits {
    pub max_entries: usize,
    pub max_entry_size: u64,
    pub max_total_expanded_size: u64,
}

impl Default for PackageLimits {
    fn default() -> Self {
        Self {
            max_entries: 256,
            max_entry_size: 8 * 1024 * 1024,
            max_total_expanded_size: 32 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Error)]
pub enum OdtError {
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("package has too many entries: {count}")]
    TooManyEntries { count: usize },
    #[error("entry is too large: {name}")]
    EntryTooLarge { name: String },
    #[error("package expanded size is too large")]
    ExpandedSizeTooLarge,
    #[error("unsafe package path: {name}")]
    UnsafePath { name: String },
    #[error("missing ODT content.xml")]
    MissingContent,
    #[error("invalid ODT mimetype")]
    InvalidMimeType,
}

pub fn write_odt_bytes(document: &Document) -> Result<Vec<u8>, OdtError> {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);

    writer.start_file(
        "mimetype",
        SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
    )?;
    writer.write_all(ODT_MIME_TYPE.as_bytes())?;

    let compressed = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    writer.start_file("content.xml", compressed)?;
    writer.write_all(render_content_xml(document).as_bytes())?;

    writer.start_file("meta.xml", compressed)?;
    writer.write_all(render_meta_xml(document).as_bytes())?;

    writer.start_file("META-INF/manifest.xml", compressed)?;
    writer.write_all(render_manifest_xml().as_bytes())?;

    let cursor = writer.finish()?;
    Ok(cursor.into_inner())
}

pub fn read_odt_bytes(bytes: &[u8]) -> Result<Document, OdtError> {
    validate_odt_package(bytes, PackageLimits::default())?;

    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;
    let mut content = String::new();
    archive
        .by_name("content.xml")
        .map_err(|_| OdtError::MissingContent)?
        .read_to_string(&mut content)?;

    let paragraphs = extract_text_paragraphs(&content);
    let mut document = Document::new_untitled();
    document.sections = vec![Section {
        blocks: paragraphs
            .into_iter()
            .map(|text| {
                Block::Paragraph(Paragraph {
                    style: StyleId::from("body"),
                    inlines: vec![Inline::text(text)],
                })
            })
            .collect(),
        ..Section::default()
    }];
    Ok(document)
}

pub fn validate_odt_package(bytes: &[u8], limits: PackageLimits) -> Result<(), OdtError> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;
    let entry_count = archive.len();
    if entry_count > limits.max_entries {
        return Err(OdtError::TooManyEntries { count: entry_count });
    }

    let mut expanded_size = 0_u64;
    let mut has_content = false;
    let mut has_mimetype = false;

    for index in 0..entry_count {
        let file = archive.by_index(index)?;
        let name = file.name().to_string();
        validate_entry_path(&name)?;

        if file.size() > limits.max_entry_size {
            return Err(OdtError::EntryTooLarge { name });
        }

        expanded_size = expanded_size.saturating_add(file.size());
        if expanded_size > limits.max_total_expanded_size {
            return Err(OdtError::ExpandedSizeTooLarge);
        }

        if name == "content.xml" {
            has_content = true;
        }
        if name == "mimetype" {
            has_mimetype = true;
        }
    }

    if !has_content {
        return Err(OdtError::MissingContent);
    }

    if has_mimetype {
        let mut mimetype = String::new();
        archive.by_name("mimetype")?.read_to_string(&mut mimetype)?;
        if mimetype != ODT_MIME_TYPE {
            return Err(OdtError::InvalidMimeType);
        }
    }

    Ok(())
}

fn validate_entry_path(name: &str) -> Result<(), OdtError> {
    if name.starts_with('/')
        || name.starts_with('\\')
        || name.contains('\\')
        || name.split('/').any(|part| part == ".." || part.is_empty())
    {
        return Err(OdtError::UnsafePath {
            name: name.to_string(),
        });
    }
    Ok(())
}

fn render_content_xml(document: &Document) -> String {
    let mut body = String::new();
    for section in &document.sections {
        for block in &section.blocks {
            match block {
                Block::Paragraph(paragraph) => {
                    body.push_str("<text:p>");
                    for inline in &paragraph.inlines {
                        body.push_str(&escape_xml(&inline.text));
                    }
                    body.push_str("</text:p>");
                }
                Block::Heading(heading) => {
                    body.push_str(&format!(
                        "<text:h text:outline-level=\"{}\">",
                        heading.level
                    ));
                    for inline in &heading.inlines {
                        body.push_str(&escape_xml(&inline.text));
                    }
                    body.push_str("</text:h>");
                }
                _ => {
                    body.push_str(
                        "<text:p>[unsupported content preserved by 900Word warning]</text:p>",
                    );
                }
            }
        }
    }

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <office:document-content xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" \
         xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" office:version=\"1.3\">\
         <office:body><office:text>{body}</office:text></office:body></office:document-content>"
    )
}

fn render_meta_xml(document: &Document) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <office:document-meta xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" \
         xmlns:dc=\"http://purl.org/dc/elements/1.1/\" office:version=\"1.3\">\
         <office:meta><dc:title>{}</dc:title><meta:generator xmlns:meta=\"urn:oasis:names:tc:opendocument:xmlns:meta:1.0\">900Word</meta:generator></office:meta>\
         </office:document-meta>",
        escape_xml(&document.meta.title)
    )
}

fn render_manifest_xml() -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\" manifest:version=\"1.3\">\
         <manifest:file-entry manifest:full-path=\"/\" manifest:media-type=\"{ODT_MIME_TYPE}\"/>\
         <manifest:file-entry manifest:full-path=\"content.xml\" manifest:media-type=\"text/xml\"/>\
         <manifest:file-entry manifest:full-path=\"meta.xml\" manifest:media-type=\"text/xml\"/>\
         </manifest:manifest>"
    )
}

fn extract_text_paragraphs(content: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("<text:p>") {
        let after_start = &rest[start + "<text:p>".len()..];
        let Some(end) = after_start.find("</text:p>") else {
            break;
        };
        paragraphs.push(unescape_xml(&strip_tags(&after_start[..end])));
        rest = &after_start[end + "</text:p>".len()..];
    }
    paragraphs
}

fn strip_tags(input: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn unescape_xml(input: &str) -> String {
    input
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_odt_round_trips_body_text() {
        let document = Document::new_untitled();

        let bytes = write_odt_bytes(&document).expect("write should succeed");
        let parsed = read_odt_bytes(&bytes).expect("read should succeed");

        assert_eq!(parsed.stats().word_count, 2);
    }

    #[test]
    fn traversal_entry_is_rejected() {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default();
        writer
            .start_file("../content.xml", options)
            .expect("test zip file should start");
        writer
            .write_all(b"<office:document-content/>")
            .expect("test zip file should write");
        let bytes = writer
            .finish()
            .expect("test zip should finish")
            .into_inner();

        let err = validate_odt_package(&bytes, PackageLimits::default())
            .expect_err("unsafe path should fail");

        assert!(matches!(err, OdtError::UnsafePath { .. }));
    }
}
