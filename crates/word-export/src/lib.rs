use std::collections::BTreeMap;
use thiserror::Error;
use word_core::{
    Block, Document, Heading, ImageBlock, Inline, InlineMark, PageField, PageRegion,
    PageRegionBlock, PageSetup, Paragraph, ParagraphFormat, Section, Style, StyleKind,
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
        push_section_regions_text(section, document, true, &mut output);
        for block in &section.blocks {
            push_block_text(block, document, &mut output);
            output.push('\n');
        }
        push_section_regions_text(section, document, false, &mut output);
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
        push_section_regions_html(section, document, true, &mut output);
        for block in &section.blocks {
            push_block_html(block, document, &mut output);
        }
        push_section_regions_html(section, document, false, &mut output);
        output.push_str("</section>");
    }

    output.push_str("</body></html>");
    Ok(output)
}

fn push_section_regions_text(
    section: &Section,
    document: &Document,
    before_body: bool,
    output: &mut String,
) {
    if before_body {
        if section.page_regions.different_first_page {
            push_region_text(
                "First page header",
                &section.page_regions.first_header,
                document,
                output,
            );
        }
        push_region_text("Header", &section.page_regions.header, document, output);
    } else {
        push_region_text("Footer", &section.page_regions.footer, document, output);
        if section.page_regions.different_first_page {
            push_region_text(
                "First page footer",
                &section.page_regions.first_footer,
                document,
                output,
            );
        }
    }
}

fn push_region_text(label: &str, region: &PageRegion, document: &Document, output: &mut String) {
    if region.blocks.is_empty() {
        return;
    }
    output.push('[');
    output.push_str(label);
    output.push_str("]\n");
    for block in &region.blocks {
        push_page_region_block_text(block, document, output);
        output.push('\n');
    }
}

fn push_page_region_block_text(block: &PageRegionBlock, document: &Document, output: &mut String) {
    match block {
        PageRegionBlock::Paragraph(paragraph) => {
            for inline in &paragraph.inlines {
                output.push_str(&inline_export_text(inline, document));
            }
        }
    }
}

fn push_block_text(block: &Block, document: &Document, output: &mut String) {
    match block {
        Block::Paragraph(paragraph) => push_paragraph_text(paragraph, document, output),
        Block::Heading(heading) => push_heading_text(heading, document, output),
        Block::List(list) => {
            for item in &list.items {
                for block in &item.blocks {
                    push_block_text(block, document, output);
                    output.push('\n');
                }
            }
        }
        Block::Table(table) => {
            for row in &table.rows {
                for cell in &row.cells {
                    for block in &cell.blocks {
                        push_block_text(block, document, output);
                    }
                    output.push('\t');
                }
                output.push('\n');
            }
        }
        Block::Image(image) => {
            if let Some(alt_text) = image
                .alt_text
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                output.push_str(alt_text);
            }
            if let Some(caption) = image
                .presentation
                .caption
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                if image
                    .alt_text
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                {
                    output.push('\n');
                }
                output.push_str(caption);
            }
        }
        Block::PageBreak => output.push_str("\n--- page break ---\n"),
    }
}

fn push_paragraph_text(paragraph: &Paragraph, document: &Document, output: &mut String) {
    for inline in &paragraph.inlines {
        output.push_str(&inline_export_text(inline, document));
    }
}

fn push_heading_text(heading: &Heading, document: &Document, output: &mut String) {
    for inline in &heading.inlines {
        output.push_str(&inline_export_text(inline, document));
    }
}

fn push_section_regions_html(
    section: &Section,
    document: &Document,
    before_body: bool,
    output: &mut String,
) {
    if before_body {
        if section.page_regions.different_first_page {
            push_region_html(
                "first-header",
                &section.page_regions.first_header,
                document,
                output,
            );
        }
        push_region_html("header", &section.page_regions.header, document, output);
    } else {
        push_region_html("footer", &section.page_regions.footer, document, output);
        if section.page_regions.different_first_page {
            push_region_html(
                "first-footer",
                &section.page_regions.first_footer,
                document,
                output,
            );
        }
    }
}

fn push_region_html(kind: &str, region: &PageRegion, document: &Document, output: &mut String) {
    if region.blocks.is_empty() {
        return;
    }
    let tag = if kind.ends_with("footer") {
        "footer"
    } else {
        "header"
    };
    output.push('<');
    output.push_str(tag);
    output.push_str(" data-page-region=\"");
    output.push_str(kind);
    output.push_str("\">");
    for block in &region.blocks {
        match block {
            PageRegionBlock::Paragraph(paragraph) => {
                output.push_str("<p>");
                push_inlines_html(&paragraph.inlines, document, output);
                output.push_str("</p>");
            }
        }
    }
    output.push_str("</");
    output.push_str(tag);
    output.push('>');
}

fn push_block_html(block: &Block, document: &Document, output: &mut String) {
    match block {
        Block::Paragraph(paragraph) => {
            output.push_str("<p");
            output.push_str(&paragraph_html_attrs(paragraph, &document.styles));
            output.push('>');
            push_inlines_html(&paragraph.inlines, document, output);
            output.push_str("</p>");
        }
        Block::Heading(heading) => {
            let level = heading.level.clamp(1, 6);
            output.push_str(&format!("<h{level}>"));
            push_inlines_html(&heading.inlines, document, output);
            output.push_str(&format!("</h{level}>"));
        }
        Block::List(list) => {
            let tag = if document
                .lists
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
                    push_block_html(block, document, output);
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
                        push_block_html(block, document, output);
                    }
                    output.push_str("</td>");
                }
                output.push_str("</tr>");
            }
            output.push_str("</table>");
        }
        Block::Image(image) => {
            let alt_text = image.alt_text.as_deref().unwrap_or("Image");
            let caption = image
                .presentation
                .caption
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(alt_text);
            output.push_str("<figure data-asset=\"");
            output.push_str(&escape_html(&image.asset_id));
            output.push('"');
            output.push_str(&image_html_attrs(image));
            output.push_str("\">");
            if let Some(data_url) = image_data_url(document, &image.asset_id) {
                output.push_str("<img src=\"");
                output.push_str(&data_url);
                output.push_str("\" alt=\"");
                output.push_str(&escape_html(alt_text));
                output.push_str("\">");
            }
            output.push_str("<figcaption>");
            output.push_str(&escape_html(caption));
            output.push_str("</figcaption>");
            output.push_str("</figure>");
        }
        Block::PageBreak => output.push_str("<hr data-page-break=\"true\">"),
    }
}

fn image_html_attrs(image: &ImageBlock) -> String {
    let mut attrs = String::new();
    let alignment = match image.presentation.alignment {
        word_core::ImageAlignment::Inline => "inline",
        word_core::ImageAlignment::Left => "left",
        word_core::ImageAlignment::Center => "center",
        word_core::ImageAlignment::Right => "right",
    };
    attrs.push_str(" data-align=\"");
    attrs.push_str(alignment);
    attrs.push('"');

    let scale = image.presentation.scale_percent.clamp(25, 200);
    attrs.push_str(" data-scale=\"");
    attrs.push_str(&scale.to_string());
    attrs.push('"');
    attrs.push_str(" style=\"");
    match image.presentation.alignment {
        word_core::ImageAlignment::Inline => {
            attrs.push_str("display:inline-block;");
        }
        word_core::ImageAlignment::Left => {
            attrs.push_str("margin-left:0;margin-right:auto;");
        }
        word_core::ImageAlignment::Center => {
            attrs.push_str("margin-left:auto;margin-right:auto;");
        }
        word_core::ImageAlignment::Right => {
            attrs.push_str("margin-left:auto;margin-right:0;");
        }
    }
    attrs.push_str("max-width:");
    attrs.push_str(&scale.to_string());
    attrs.push_str("%;\"");
    attrs
}

fn image_data_url(document: &Document, asset_id: &str) -> Option<String> {
    let asset = document.assets.get(asset_id)?;
    let detected = detect_image_media_type(&asset.bytes)?;
    if asset.byte_len != asset.bytes.len() || asset.media_type != detected {
        return None;
    }
    Some(format!(
        "data:{};base64,{}",
        detected,
        base64_encode(&asset.bytes)
    ))
}

fn detect_image_media_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = *chunk.get(1).unwrap_or(&0);
        let third = *chunk.get(2).unwrap_or(&0);
        let value = ((first as u32) << 16) | ((second as u32) << 8) | third as u32;
        output.push(ALPHABET[((value >> 18) & 0x3f) as usize] as char);
        output.push(ALPHABET[((value >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            output.push(ALPHABET[((value >> 6) & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(ALPHABET[(value & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

fn push_inlines_html(inlines: &[Inline], document: &Document, output: &mut String) {
    for inline in inlines {
        push_inline_html(inline, document, output);
    }
}

fn push_inline_html(inline: &Inline, document: &Document, output: &mut String) {
    if let Some(field) = inline.field {
        output.push_str("<span data-page-field=\"");
        output.push_str(page_field_name(field));
        output.push_str("\">");
        output.push_str(&escape_html(&inline_export_text(inline, document)));
        output.push_str("</span>");
        return;
    }

    let safe_href = inline.link.as_deref().and_then(sanitize_href);
    if let Some(href) = safe_href {
        output.push_str("<a rel=\"noreferrer\" href=\"");
        output.push_str(&escape_html(href));
        output.push_str("\">");
    }

    let styled_span = !inline.style.is_default();
    if styled_span {
        output.push_str("<span style=\"");
        if let Some(font_family) = inline.style.font_family.as_deref() {
            output.push_str("font-family:");
            output.push_str(&escape_html(font_family));
            output.push(';');
        }
        if let Some(font_size) = inline.style.font_size_pt {
            output.push_str("font-size:");
            output.push_str(&font_size.to_string());
            output.push_str("pt;");
        }
        if let Some(text_color) = inline.style.text_color.as_deref() {
            output.push_str("color:");
            output.push_str(&escape_html(text_color));
            output.push(';');
        }
        if let Some(highlight_color) = inline.style.highlight_color.as_deref() {
            output.push_str("background-color:");
            output.push_str(&escape_html(highlight_color));
            output.push(';');
        }
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

    output.push_str(&escape_html(&inline_export_text(inline, document)));

    for tag in opened_marks.into_iter().rev() {
        output.push_str("</");
        output.push_str(tag);
        output.push('>');
    }

    if styled_span {
        output.push_str("</span>");
    }

    if safe_href.is_some() {
        output.push_str("</a>");
    }
}

fn inline_export_text(inline: &Inline, document: &Document) -> String {
    match inline.field {
        Some(PageField::PageNumber) => "1".to_string(),
        Some(PageField::PageCount) => "1".to_string(),
        Some(PageField::Date) => document.meta.modified_at.format("%Y-%m-%d").to_string(),
        None => inline.text.clone(),
    }
}

fn page_field_name(field: PageField) -> &'static str {
    match field {
        PageField::PageNumber => "page-number",
        PageField::PageCount => "page-count",
        PageField::Date => "date",
    }
}

fn paragraph_html_attrs(
    paragraph: &Paragraph,
    styles: &BTreeMap<word_core::StyleId, Style>,
) -> String {
    let mut attrs = String::new();
    attrs.push_str(" data-style=\"");
    attrs.push_str(&escape_html(paragraph.style.as_str()));
    attrs.push('"');
    let format = effective_paragraph_format(paragraph, styles);
    if format.is_default() {
        return attrs;
    }

    let mut style = String::new();
    if let Some(alignment) = format.alignment {
        let value = match alignment {
            word_core::ParagraphAlignment::Left => "left",
            word_core::ParagraphAlignment::Center => "center",
            word_core::ParagraphAlignment::Right => "right",
            word_core::ParagraphAlignment::Justify => "justify",
        };
        style.push_str("text-align:");
        style.push_str(value);
        style.push(';');
    }
    if let Some(line_spacing) = format.line_spacing_per_mille {
        style.push_str("line-height:");
        style.push_str(&(line_spacing as f32 / 1000.0).to_string());
        style.push(';');
    }
    if let Some(spacing_before) = format.spacing_before_mm {
        style.push_str("margin-top:");
        style.push_str(&spacing_before.to_string());
        style.push_str("mm;");
    }
    if let Some(spacing_after) = format.spacing_after_mm {
        style.push_str("margin-bottom:");
        style.push_str(&spacing_after.to_string());
        style.push_str("mm;");
    }
    if let Some(indent_start) = format.indent_start_mm {
        style.push_str("margin-left:");
        style.push_str(&indent_start.to_string());
        style.push_str("mm;");
    }
    if let Some(indent_end) = format.indent_end_mm {
        style.push_str("margin-right:");
        style.push_str(&indent_end.to_string());
        style.push_str("mm;");
    }
    if let Some(first_line_indent) = format.first_line_indent_mm {
        style.push_str("text-indent:");
        style.push_str(&first_line_indent.to_string());
        style.push_str("mm;");
    }
    if !style.is_empty() {
        attrs.push_str(" style=\"");
        attrs.push_str(&escape_html(&style));
        attrs.push('"');
    }
    attrs
}

fn effective_paragraph_format(
    paragraph: &Paragraph,
    styles: &BTreeMap<word_core::StyleId, Style>,
) -> ParagraphFormat {
    let mut format = styles
        .get(&paragraph.style)
        .filter(|style| style.kind == StyleKind::Paragraph)
        .and_then(|style| style.properties.paragraph.clone())
        .unwrap_or_default();

    if paragraph.format.alignment.is_some() {
        format.alignment = paragraph.format.alignment;
    }
    if paragraph.format.line_spacing_per_mille.is_some() {
        format.line_spacing_per_mille = paragraph.format.line_spacing_per_mille;
    }
    if paragraph.format.spacing_before_mm.is_some() {
        format.spacing_before_mm = paragraph.format.spacing_before_mm;
    }
    if paragraph.format.spacing_after_mm.is_some() {
        format.spacing_after_mm = paragraph.format.spacing_after_mm;
    }
    if paragraph.format.indent_start_mm.is_some() {
        format.indent_start_mm = paragraph.format.indent_start_mm;
    }
    if paragraph.format.indent_end_mm.is_some() {
        format.indent_end_mm = paragraph.format.indent_end_mm;
    }
    if paragraph.format.first_line_indent_mm.is_some() {
        format.first_line_indent_mm = paragraph.format.first_line_indent_mm;
    }
    format
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
    use word_core::{
        ImageAlignment, ImageBlock, ImagePresentation, ListBlock, ListDefinition, ListItem,
        PageRegionParagraph, Table, TableCell, TableRow,
    };

    #[test]
    fn txt_export_preserves_empty_document_text() {
        let document = Document::new_untitled();

        assert_eq!(
            export_txt(&document).expect("txt export should succeed"),
            ""
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
                        format: Default::default(),
                        inlines: vec![Inline::text("List item")],
                    })],
                }],
            }),
            Block::Table(Table {
                rows: vec![TableRow {
                    cells: vec![TableCell {
                        blocks: vec![Block::Paragraph(Paragraph {
                            style: "body".into(),
                            format: Default::default(),
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
    fn exports_include_page_regions_and_render_fields() {
        let mut document = Document::new_untitled();
        document.sections[0].page_regions.header.blocks =
            vec![PageRegionBlock::Paragraph(PageRegionParagraph {
                inlines: vec![
                    Inline::text("Header "),
                    Inline::field(PageField::PageNumber),
                    Inline::text("/"),
                    Inline::field(PageField::PageCount),
                ],
            })];
        document.sections[0].page_regions.footer.blocks =
            vec![PageRegionBlock::Paragraph(PageRegionParagraph {
                inlines: vec![Inline::text("Footer "), Inline::field(PageField::Date)],
            })];

        let text = export_txt(&document).expect("txt export should succeed");
        let html = export_html(&document).expect("html export should succeed");
        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");

        assert!(text.contains("[Header]\nHeader 1/1"));
        assert!(text.contains("[Footer]\nFooter "));
        assert!(html.contains("data-page-region=\"header\""));
        assert!(html.contains("data-page-field=\"page-number\">1</span>"));
        assert!(html.contains("data-page-field=\"date\""));
        assert!(pdf
            .windows("Header 1/1".len())
            .any(|window| window == b"Header 1/1"));
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
            format: Default::default(),
            inlines: vec![Inline {
                text: "unsafe".to_string(),
                marks: vec![InlineMark::Bold],
                link: Some("javascript:alert(1)".to_string()),
                style: Default::default(),
                field: None,
            }],
        })];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("<strong>unsafe</strong>"));
        assert!(!html.contains("javascript:"));
        assert!(!html.contains("<script"));
    }

    #[test]
    fn html_export_preserves_authoring_direct_formatting() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            style: "quote".into(),
            format: word_core::ParagraphFormat {
                alignment: Some(word_core::ParagraphAlignment::Center),
                line_spacing_per_mille: Some(1500),
                spacing_before_mm: None,
                spacing_after_mm: Some(4),
                indent_start_mm: None,
                indent_end_mm: None,
                first_line_indent_mm: None,
            },
            inlines: vec![Inline {
                text: "Styled".to_string(),
                marks: vec![],
                link: None,
                style: word_core::InlineStyle {
                    font_family: Some("serif".to_string()),
                    font_size_pt: Some(14),
                    text_color: Some("#1f2937".to_string()),
                    highlight_color: Some("#fff3bf".to_string()),
                },
                field: None,
            }],
        })];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("data-style=\"quote\""));
        assert!(html.contains("text-align:center"));
        assert!(html.contains("line-height:1.5"));
        assert!(html.contains("font-family:serif"));
        assert!(html.contains("background-color:#fff3bf"));
    }

    #[test]
    fn html_export_applies_paragraph_style_properties() {
        let mut document = Document::new_untitled();
        document
            .register_style(Style {
                id: "quote".into(),
                name: "Quote".to_string(),
                kind: StyleKind::Paragraph,
                parent: None,
                properties: word_core::StyleProperties {
                    paragraph: Some(word_core::ParagraphFormat {
                        alignment: Some(word_core::ParagraphAlignment::Justify),
                        line_spacing_per_mille: Some(1500),
                        spacing_before_mm: Some(0),
                        spacing_after_mm: Some(4),
                        indent_start_mm: Some(6),
                        indent_end_mm: None,
                        first_line_indent_mm: Some(-2),
                    }),
                    inline: None,
                    page: None,
                },
            })
            .expect("style should register");
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            style: "quote".into(),
            format: Default::default(),
            inlines: vec![Inline::text("Styled")],
        })];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("data-style=\"quote\""));
        assert!(html.contains("text-align:justify"));
        assert!(html.contains("line-height:1.5"));
        assert!(html.contains("margin-top:0mm"));
        assert!(html.contains("margin-left:6mm"));
        assert!(html.contains("text-indent:-2mm"));
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
                    format: Default::default(),
                    inlines: vec![Inline::text("First")],
                })],
            }],
        })];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("<ol><li><p data-style=\"body\">First</p></li></ol>"));
    }

    #[test]
    fn html_export_does_not_emit_remote_images_or_handlers() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Image(ImageBlock {
            asset_id: "asset-1".to_string(),
            presentation: ImagePresentation::default(),
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
    fn html_export_embeds_allowlisted_image_asset_as_data_url() {
        let mut document = Document::new_untitled();
        document.assets.insert(
            "image-1.png".to_string(),
            word_core::AssetRef {
                id: "image-1.png".to_string(),
                media_type: "image/png".to_string(),
                byte_len: 8,
                bytes: b"\x89PNG\r\n\x1a\n".to_vec(),
                original_name: None,
            },
        );
        document.sections[0].blocks = vec![Block::Image(ImageBlock {
            asset_id: "image-1.png".to_string(),
            presentation: ImagePresentation {
                alignment: ImageAlignment::Center,
                scale_percent: 75,
                caption: Some("Centered caption".to_string()),
            },
            alt_text: Some("Image".to_string()),
        })];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("<img src=\"data:image/png;base64,iVBORw0KGgo=\" alt=\"Image\">"));
        assert!(html.contains("data-align=\"center\""));
        assert!(html.contains("data-scale=\"75\""));
        assert!(html.contains("max-width:75%"));
        assert!(html.contains("<figcaption>Centered caption</figcaption>"));
        assert!(!html.contains("file://"));
        assert!(!html.contains("private"));
    }

    #[test]
    fn html_export_does_not_embed_mislabeled_image_asset() {
        let mut document = Document::new_untitled();
        document.assets.insert(
            "image-1.png".to_string(),
            word_core::AssetRef {
                id: "image-1.png".to_string(),
                media_type: "image/png".to_string(),
                byte_len: 4,
                bytes: b"HTML".to_vec(),
                original_name: None,
            },
        );
        document.sections[0].blocks = vec![Block::Image(ImageBlock {
            asset_id: "image-1.png".to_string(),
            presentation: ImagePresentation::default(),
            alt_text: Some("Image".to_string()),
        })];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("<figure data-asset=\"image-1.png\""));
        assert!(!html.contains("<img"));
        assert!(!html.contains("data:image/"));
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
        assert!(!pdf
            .windows("Start writing...".len())
            .any(|window| window == b"Start writing..."));
    }
}
