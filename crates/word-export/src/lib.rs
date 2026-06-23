use std::collections::BTreeMap;
use thiserror::Error;
use word_core::{
    Block, Document, Heading, Inline, InlineMark, ListDefinition, PageSetup, Paragraph,
};

const POINTS_PER_MM: f32 = 72.0 / 25.4;
const PDF_FONT_SIZE: f32 = 11.0;
const PDF_LEADING: f32 = 14.0;
const PDF_MIN_MARGIN_POINTS: f32 = 24.0;

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
    export_html_with_options(document, HtmlExportOptions::default())
}

pub fn export_print_html(document: &Document) -> Result<String, ExportError> {
    export_html_with_options(document, HtmlExportOptions { print_ready: true })
}

pub fn export_basic_pdf(document: &Document) -> Result<Vec<u8>, ExportError> {
    if document.sections.is_empty() {
        return Err(ExportError::EmptyDocument);
    }

    let default_page = PageSetup::default();
    let page = document
        .sections
        .first()
        .map(|section| &section.page)
        .unwrap_or(&default_page);
    let page_width = mm_to_points(page.width_mm);
    let page_height = mm_to_points(page.height_mm);
    let margin_left = mm_to_points(page.margin_left_mm).max(PDF_MIN_MARGIN_POINTS);
    let margin_top = mm_to_points(page.margin_top_mm).max(PDF_MIN_MARGIN_POINTS);
    let start_y = (page_height - margin_top).max(PDF_MIN_MARGIN_POINTS);
    let lines = pdf_lines(document)?;

    let mut stream = format!(
        "BT /F1 {PDF_FONT_SIZE:.1} Tf {margin_left:.1} {start_y:.1} Td {PDF_LEADING:.1} TL\n"
    );
    for line in lines {
        stream.push('(');
        stream.push_str(&escape_pdf_text(&line));
        stream.push_str(") Tj T*\n");
    }
    stream.push_str("ET");

    Ok(build_pdf(page_width, page_height, &stream))
}

#[derive(Debug, Clone, Copy, Default)]
struct HtmlExportOptions {
    print_ready: bool,
}

fn export_html_with_options(
    document: &Document,
    options: HtmlExportOptions,
) -> Result<String, ExportError> {
    if document.sections.is_empty() {
        return Err(ExportError::EmptyDocument);
    }

    let mut output = String::from("<!doctype html>\n<html><head><meta charset=\"utf-8\">");
    output.push_str(
        "<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; img-src data:; base-uri 'none'; form-action 'none'\">",
    );
    output.push_str("<meta name=\"generator\" content=\"900Word\"><title>");
    output.push_str(&escape_html(&document.meta.title));
    output.push_str("</title>");
    push_export_css(document, options, &mut output);
    output.push_str("</head><body>");

    for section in &document.sections {
        output.push_str("<section>");
        for block in &section.blocks {
            push_block_html(block, &document.lists, &mut output);
        }
        output.push_str("</section>");
    }

    output.push_str("</body></html>");
    Ok(output)
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

fn push_block_html(block: &Block, lists: &BTreeMap<String, ListDefinition>, output: &mut String) {
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
            let tag = if lists
                .get(&list.definition_id)
                .map(|definition| definition.ordered)
                .unwrap_or(false)
            {
                "ol"
            } else {
                "ul"
            };
            output.push('<');
            output.push_str(tag);
            output.push('>');
            for item in &list.items {
                output.push_str("<li>");
                for block in &item.blocks {
                    push_block_html(block, lists, output);
                }
                output.push_str("</li>");
            }
            output.push_str("</");
            output.push_str(tag);
            output.push('>');
        }
        Block::Table(table) => {
            output.push_str("<table>");
            for row in &table.rows {
                output.push_str("<tr>");
                for cell in &row.cells {
                    output.push_str("<td>");
                    for block in &cell.blocks {
                        push_block_html(block, lists, output);
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
        push_inline_html(inline, output);
    }
}

fn push_inline_html(inline: &Inline, output: &mut String) {
    let safe_href = inline.link.as_deref().and_then(sanitize_href);
    if let Some(href) = safe_href {
        output.push_str("<a rel=\"noreferrer\" href=\"");
        output.push_str(&escape_html(href));
        output.push_str("\">");
    }

    let mut opened_marks = Vec::new();
    for mark in &inline.marks {
        let tag = mark_html_tag(*mark);
        output.push('<');
        output.push_str(tag);
        output.push('>');
        opened_marks.push(tag);
    }

    output.push_str(&escape_html(&inline.text));

    for tag in opened_marks.into_iter().rev() {
        output.push_str("</");
        output.push_str(tag);
        output.push('>');
    }

    if safe_href.is_some() {
        output.push_str("</a>");
    }
}

fn mark_html_tag(mark: InlineMark) -> &'static str {
    match mark {
        InlineMark::Bold => "strong",
        InlineMark::Italic => "em",
        InlineMark::Underline => "u",
        InlineMark::Strikethrough => "s",
        InlineMark::Superscript => "sup",
        InlineMark::Subscript => "sub",
    }
}

fn sanitize_href(href: &str) -> Option<&str> {
    let trimmed = href.trim();
    let lowercase = trimmed.to_ascii_lowercase();
    if lowercase.starts_with("https://")
        || lowercase.starts_with("http://")
        || lowercase.starts_with("mailto:")
    {
        Some(trimmed)
    } else {
        None
    }
}

fn push_export_css(document: &Document, options: HtmlExportOptions, output: &mut String) {
    let page = document
        .sections
        .first()
        .map(|section| &section.page)
        .cloned()
        .unwrap_or_default();
    output.push_str("<style>");
    output.push_str("body{font-family:system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;line-height:1.55;color:#1d2433;background:#fff;margin:2rem;}");
    output.push_str("section{max-width:48rem;margin:0 auto;}table{border-collapse:collapse;width:100%;}td{border:1px solid #9aa7b8;padding:.35rem;vertical-align:top;}figure{margin:1rem 0;padding:.75rem;border:1px solid #d6dce5;}figcaption{color:#526070;}a{color:#0b63b6;}hr[data-page-break=\"true\"]{break-after:page;border:0;border-top:1px dashed #9aa7b8;}");
    if options.print_ready {
        output.push_str(&format!(
            "@page{{size:{}mm {}mm;margin:{}mm {}mm {}mm {}mm;}}body{{margin:0;}}section{{max-width:none;}}",
            page.width_mm,
            page.height_mm,
            page.margin_top_mm,
            page.margin_right_mm,
            page.margin_bottom_mm,
            page.margin_left_mm
        ));
    }
    output.push_str("</style>");
}

fn pdf_lines(document: &Document) -> Result<Vec<String>, ExportError> {
    let text = export_txt(document)?;
    let mut lines = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            lines.push(String::new());
            continue;
        }
        for chunk in wrap_pdf_line(line, 92) {
            lines.push(chunk);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    Ok(lines)
}

fn wrap_pdf_line(line: &str, limit: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.chars().count() + 1 + word.chars().count() <= limit {
            current.push(' ');
            current.push_str(word);
        } else {
            chunks.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn escape_pdf_text(input: &str) -> String {
    let mut output = String::new();
    for ch in input.chars() {
        match ch {
            '\\' => output.push_str("\\\\"),
            '(' => output.push_str("\\("),
            ')' => output.push_str("\\)"),
            '\t' => output.push_str("    "),
            ch if ch.is_ascii_graphic() || ch == ' ' => output.push(ch),
            _ => output.push('?'),
        }
    }
    output
}

fn build_pdf(page_width: f32, page_height: f32, stream: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_width:.1} {page_height:.1}] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", stream.len(), stream),
    ];

    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = Vec::with_capacity(objects.len() + 1);
    offsets.push(0);
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.push_str(&format!("{} 0 obj\n{}\nendobj\n", index + 1, object));
    }
    let xref_start = pdf.len();
    pdf.push_str(&format!("xref\n0 {}\n", objects.len() + 1));
    pdf.push_str("0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.push_str(&format!("{offset:010} 00000 n \n"));
    }
    pdf.push_str(&format!(
        "trailer << /Size {} /Root 1 0 R >>\nstartxref\n{xref_start}\n%%EOF\n",
        objects.len() + 1
    ));
    pdf.into_bytes()
}

fn mm_to_points(value: u16) -> f32 {
    value as f32 * POINTS_PER_MM
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
    use word_core::{ImageBlock, ListBlock, ListDefinition, ListItem, Table, TableCell, TableRow};

    #[test]
    fn txt_export_contains_body_text() {
        let document = Document::new_untitled();

        assert_eq!(
            export_txt(&document).expect("txt export should succeed"),
            "Start writing..."
        );
    }

    #[test]
    fn txt_export_covers_lists_tables_and_page_breaks() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::List(ListBlock {
                definition_id: "unordered".to_string(),
                items: vec![ListItem {
                    level: 0,
                    blocks: vec![Block::Paragraph(Paragraph {
                        style: "body".into(),
                        inlines: vec![Inline::text("List item")],
                    })],
                }],
            }),
            Block::Table(Table {
                rows: vec![TableRow {
                    cells: vec![TableCell {
                        blocks: vec![Block::Paragraph(Paragraph {
                            style: "body".into(),
                            inlines: vec![Inline::text("Cell text")],
                        })],
                    }],
                }],
            }),
            Block::PageBreak,
        ];

        let text = export_txt(&document).expect("txt export should succeed");

        assert!(text.contains("List item"));
        assert!(text.contains("Cell text"));
        assert!(text.contains("--- page break ---"));
    }

    #[test]
    fn html_export_escapes_text() {
        let mut document = Document::new_untitled();
        document.meta.title = "<Draft>".to_string();

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("&lt;Draft&gt;"));
    }

    #[test]
    fn html_export_strips_unsafe_links_and_preserves_marks() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            style: "body".into(),
            inlines: vec![Inline {
                text: "unsafe".to_string(),
                marks: vec![InlineMark::Bold],
                link: Some("javascript:alert(1)".to_string()),
            }],
        })];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("<strong>unsafe</strong>"));
        assert!(!html.contains("javascript:"));
        assert!(!html.contains("<script"));
    }

    #[test]
    fn html_export_uses_list_definitions_for_ordered_lists() {
        let mut document = Document::new_untitled();
        document.lists.insert(
            "numbers".to_string(),
            ListDefinition {
                ordered: true,
                marker: None,
            },
        );
        document.sections[0].blocks = vec![Block::List(ListBlock {
            definition_id: "numbers".to_string(),
            items: vec![ListItem {
                level: 0,
                blocks: vec![Block::Paragraph(Paragraph {
                    style: "body".into(),
                    inlines: vec![Inline::text("First")],
                })],
            }],
        })];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("<ol><li><p>First</p></li></ol>"));
    }

    #[test]
    fn html_export_does_not_emit_remote_images_or_handlers() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Image(ImageBlock {
            asset_id: "asset-1".to_string(),
            alt_text: Some("<img src=x onerror=alert(1)>".to_string()),
        })];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("data-asset=\"asset-1\""));
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
        assert!(!html.contains(" src=\"http"));
        assert!(!html.contains("<img"));
        assert!(!html.contains("<script"));
    }

    #[test]
    fn print_html_includes_page_setup_css() {
        let mut document = Document::new_untitled();
        document.sections[0].page.width_mm = 148;
        document.sections[0].page.height_mm = 210;

        let html = export_print_html(&document).expect("print html should export");

        assert!(html.contains("@page{size:148mm 210mm;"));
        assert!(html.contains("Content-Security-Policy"));
    }

    #[test]
    fn pdf_export_returns_pdf_header() {
        let document = Document::new_untitled();

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");

        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        assert!(pdf.windows(4).any(|window| window == b"xref"));
        assert!(pdf
            .windows("Start writing...".len())
            .any(|window| window == b"Start writing..."));
    }
}
