use thiserror::Error;
use word_core::{Block, Document, Heading, Inline, Paragraph};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExportError {
    #[error("document has no sections")]
    EmptyDocument,
}

pub fn export_txt(document: &Document) -> Result<String, ExportError> {
    if document.sections.is_empty() {
        return Err(ExportError::EmptyDocument);
    }

    let mut output = String::new();
    for section in &document.sections {
        for block in &section.blocks {
            push_block_text(block, &mut output);
            output.push('\n');
        }
    }
    Ok(output.trim_end().to_string())
}

pub fn export_html(document: &Document) -> Result<String, ExportError> {
    if document.sections.is_empty() {
        return Err(ExportError::EmptyDocument);
    }

    let mut output = String::from("<!doctype html><html><head><meta charset=\"utf-8\"><title>");
    output.push_str(&escape_html(&document.meta.title));
    output.push_str("</title></head><body>");

    for section in &document.sections {
        for block in &section.blocks {
            push_block_html(block, &mut output);
        }
    }

    output.push_str("</body></html>");
    Ok(output)
}

pub fn export_basic_pdf(document: &Document) -> Result<Vec<u8>, ExportError> {
    let text = export_txt(document)?;
    let escaped = text
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)");
    let stream = format!("BT /F1 12 Tf 72 720 Td ({escaped}) Tj ET");
    let content_len = stream.len();
    let pdf = format!(
        "%PDF-1.4\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n\
         2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n\
         3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >> endobj\n\
         4 0 obj << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> endobj\n\
         5 0 obj << /Length {content_len} >> stream\n{stream}\nendstream endobj\n\
         trailer << /Root 1 0 R >>\n%%EOF\n"
    );
    Ok(pdf.into_bytes())
}

fn push_block_text(block: &Block, output: &mut String) {
    match block {
        Block::Paragraph(paragraph) => push_paragraph_text(paragraph, output),
        Block::Heading(heading) => push_heading_text(heading, output),
        Block::List(list) => {
            for item in &list.items {
                for block in &item.blocks {
                    push_block_text(block, output);
                    output.push('\n');
                }
            }
        }
        Block::Table(table) => {
            for row in &table.rows {
                for cell in &row.cells {
                    for block in &cell.blocks {
                        push_block_text(block, output);
                    }
                    output.push('\t');
                }
                output.push('\n');
            }
        }
        Block::Image(image) => {
            if let Some(alt_text) = &image.alt_text {
                output.push_str(alt_text);
            }
        }
        Block::PageBreak => output.push_str("\n--- page break ---\n"),
    }
}

fn push_paragraph_text(paragraph: &Paragraph, output: &mut String) {
    for inline in &paragraph.inlines {
        output.push_str(&inline.text);
    }
}

fn push_heading_text(heading: &Heading, output: &mut String) {
    for inline in &heading.inlines {
        output.push_str(&inline.text);
    }
}

fn push_block_html(block: &Block, output: &mut String) {
    match block {
        Block::Paragraph(paragraph) => {
            output.push_str("<p>");
            push_inlines_html(&paragraph.inlines, output);
            output.push_str("</p>");
        }
        Block::Heading(heading) => {
            let level = heading.level.clamp(1, 6);
            output.push_str(&format!("<h{level}>"));
            push_inlines_html(&heading.inlines, output);
            output.push_str(&format!("</h{level}>"));
        }
        Block::List(list) => {
            output.push_str("<ul>");
            for item in &list.items {
                output.push_str("<li>");
                for block in &item.blocks {
                    push_block_html(block, output);
                }
                output.push_str("</li>");
            }
            output.push_str("</ul>");
        }
        Block::Table(table) => {
            output.push_str("<table>");
            for row in &table.rows {
                output.push_str("<tr>");
                for cell in &row.cells {
                    output.push_str("<td>");
                    for block in &cell.blocks {
                        push_block_html(block, output);
                    }
                    output.push_str("</td>");
                }
                output.push_str("</tr>");
            }
            output.push_str("</table>");
        }
        Block::Image(image) => {
            output.push_str("<figure data-asset=\"");
            output.push_str(&escape_html(&image.asset_id));
            output.push_str("\">");
            if let Some(alt_text) = &image.alt_text {
                output.push_str("<figcaption>");
                output.push_str(&escape_html(alt_text));
                output.push_str("</figcaption>");
            }
            output.push_str("</figure>");
        }
        Block::PageBreak => output.push_str("<hr data-page-break=\"true\">"),
    }
}

fn push_inlines_html(inlines: &[Inline], output: &mut String) {
    for inline in inlines {
        output.push_str(&escape_html(&inline.text));
    }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use word_core::Document;

    #[test]
    fn txt_export_contains_body_text() {
        let document = Document::new_untitled();

        assert_eq!(
            export_txt(&document).expect("txt export should succeed"),
            "Start writing..."
        );
    }

    #[test]
    fn html_export_escapes_text() {
        let mut document = Document::new_untitled();
        document.meta.title = "<Draft>".to_string();

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("&lt;Draft&gt;"));
    }

    #[test]
    fn pdf_export_returns_pdf_header() {
        let document = Document::new_untitled();

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");

        assert!(pdf.starts_with(b"%PDF-1.4"));
    }
}
