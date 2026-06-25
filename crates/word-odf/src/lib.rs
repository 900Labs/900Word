use chrono::{DateTime, Utc};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Display;
use std::io::{Cursor, Read, Write};
use thiserror::Error;
use word_core::{
    normalize_comment_author, sanitize_table_cell_background_color, validate_comment_body,
    validate_comment_id, validate_note_body, validate_note_id, validate_note_label,
    validate_note_reference, validate_tracked_change_id, AssetRef, Block, CommentThread, Document,
    DocumentWarning, Heading, ImageAlignment, ImageBlock, ImagePresentation, Inline, InlineMark,
    InlineNoteReference, InlineStyle, ListBlock, ListDefinition, ListItem, Note, NoteKind,
    PageField, PageRegion, PageRegionBlock, PageRegionKind, PageRegionParagraph, PageRegions,
    PageSetup, Paragraph, ParagraphAlignment, ParagraphFormat, Section, Style, StyleId, StyleKind,
    Table, TableCell, TableCellBorder, TableCellPresentation, TableOfContents,
    TableOfContentsEntry, TableRow, TrackChangesState, TrackedChange, TrackedChangeKind,
};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const ODT_MIME_TYPE: &str = "application/vnd.oasis.opendocument.text";
const TEXT_STYLE_PREFIX: &str = "900w";
const PARAGRAPH_STYLE_PREFIX: &str = "900wp";
const ORDERED_LIST_STYLE: &str = "900w-ordered";
const UNORDERED_LIST_STYLE: &str = "900w-unordered";
const IMAGE_PARAGRAPH_STYLE: &str = "900w-image";
const TOC_PARAGRAPH_STYLE: &str = "900w-toc";
const PAGE_LAYOUT_STYLE: &str = "900w-page-layout";
const MASTER_PAGE_STYLE: &str = "900w-master-page";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackageLimits {
    pub max_package_size: u64,
    pub max_entries: usize,
    pub max_entry_size: u64,
    pub max_total_expanded_size: u64,
    pub max_path_depth: usize,
    pub max_xml_depth: usize,
    pub max_image_size: u64,
}

impl Default for PackageLimits {
    fn default() -> Self {
        Self {
            max_package_size: 64 * 1024 * 1024,
            max_entries: 256,
            max_entry_size: 8 * 1024 * 1024,
            max_total_expanded_size: 32 * 1024 * 1024,
            max_path_depth: 8,
            max_xml_depth: 128,
            max_image_size: 8 * 1024 * 1024,
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
    #[error("package is too large")]
    PackageTooLarge,
    #[error("package path is too deep: {name}")]
    PathTooDeep { name: String },
    #[error("image entry is too large: {name}")]
    ImageTooLarge { name: String },
    #[error("unsupported or unsafe image type: {name}")]
    UnsupportedImageType { name: String },
    #[error("package expanded size is too large")]
    ExpandedSizeTooLarge,
    #[error("unsafe package path: {name}")]
    UnsafePath { name: String },
    #[error("symlink package entry is not allowed: {name}")]
    SymlinkEntry { name: String },
    #[error("encrypted package entry is not allowed: {name}")]
    EncryptedEntry { name: String },
    #[error("unexpected executable package entry: {name}")]
    ExecutableEntry { name: String },
    #[error("missing ODT content.xml")]
    MissingContent,
    #[error("invalid ODT mimetype")]
    InvalidMimeType,
    #[error("missing image asset: {asset_id}")]
    MissingAsset { asset_id: String },
    #[error("unsafe image asset name: {asset_id}")]
    UnsafeAssetName { asset_id: String },
    #[error("document contains imported read-only header or footer content")]
    ReadOnlyPageRegion,
    #[error("xml error in {name}: {message}")]
    Xml { name: String, message: String },
    #[error("xml depth exceeds limit in {name}")]
    XmlTooDeep { name: String },
    #[error("xml entity declarations are not allowed in {name}")]
    XmlEntityDeclaration { name: String },
}

#[derive(Debug, Clone)]
struct AssetPayload {
    id: String,
    media_type: String,
    bytes: Vec<u8>,
}

pub fn write_odt_bytes(document: &Document) -> Result<Vec<u8>, OdtError> {
    if document
        .sections
        .iter()
        .any(|section| section.page_regions.has_read_only_content())
    {
        return Err(OdtError::ReadOnlyPageRegion);
    }

    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);

    writer.start_file(
        "mimetype",
        SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
    )?;
    writer.write_all(ODT_MIME_TYPE.as_bytes())?;

    let compressed = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    writer.start_file("content.xml", compressed)?;
    writer.write_all(render_content_xml(document)?.as_bytes())?;

    writer.start_file("meta.xml", compressed)?;
    writer.write_all(render_meta_xml(document).as_bytes())?;

    writer.start_file("styles.xml", compressed)?;
    writer.write_all(render_styles_xml(document).as_bytes())?;

    writer.start_file("META-INF/manifest.xml", compressed)?;
    writer.write_all(render_manifest_xml(document)?.as_bytes())?;

    for asset in image_assets_in_document(document)? {
        validate_image_asset(asset)?;
        let path = asset_package_path(asset)?;
        writer.start_file(path, compressed)?;
        writer.write_all(&asset.bytes)?;
    }

    let cursor = writer.finish()?;
    Ok(cursor.into_inner())
}

pub fn read_odt_bytes(bytes: &[u8]) -> Result<Document, OdtError> {
    read_odt_bytes_with_limits(bytes, PackageLimits::default())
}

pub fn read_odt_bytes_with_limits(
    bytes: &[u8],
    limits: PackageLimits,
) -> Result<Document, OdtError> {
    validate_odt_package(bytes, limits)?;

    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;
    let mut content = String::new();
    let mut meta = String::new();
    let mut styles = String::new();
    let mut asset_payloads = BTreeMap::new();

    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        if file.is_dir() {
            continue;
        }
        let name = file.name().to_string();
        match name.as_str() {
            "content.xml" => {
                file.read_to_string(&mut content)?;
            }
            "meta.xml" => {
                file.read_to_string(&mut meta)?;
            }
            "styles.xml" => {
                file.read_to_string(&mut styles)?;
            }
            _ if name.starts_with("Pictures/") => {
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)?;
                let media_type = detect_image_media_type(&bytes)
                    .ok_or_else(|| OdtError::UnsupportedImageType { name: name.clone() })?;
                let id = generic_imported_image_id(asset_payloads.len() + 1, media_type);
                asset_payloads.insert(
                    name.clone(),
                    AssetPayload {
                        id,
                        media_type: media_type.to_string(),
                        bytes,
                    },
                );
            }
            _ => {}
        }
    }

    if content.is_empty() {
        return Err(OdtError::MissingContent);
    }

    let mut document = parse_content_xml(&content, &asset_payloads)?;
    if !meta.is_empty() {
        if let Some(title) = extract_meta_title(&meta)? {
            document.meta.title = title;
        }
    }
    if !styles.is_empty() {
        let parsed_regions = parse_page_regions_xml(&styles)?;
        if let Some(section) = document.sections.first_mut() {
            section.page_regions = parsed_regions.regions;
        }
        document.warnings.extend(parsed_regions.warnings);
    }
    Ok(document)
}

pub fn validate_odt_package(bytes: &[u8], limits: PackageLimits) -> Result<(), OdtError> {
    if bytes.len() as u64 > limits.max_package_size {
        return Err(OdtError::PackageTooLarge);
    }

    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;
    let entry_count = archive.len();
    if entry_count > limits.max_entries {
        return Err(OdtError::TooManyEntries { count: entry_count });
    }

    let mut expanded_size = 0_u64;
    let mut has_content = false;
    let mut has_valid_mimetype = false;

    for index in 0..entry_count {
        let mut file = archive.by_index(index)?;
        let name = file.name().to_string();
        validate_entry_path(&name, limits)?;
        if index == 0 && name != "mimetype" {
            return Err(OdtError::InvalidMimeType);
        }
        validate_entry_mode(&file, &name)?;
        validate_entry_kind(&name)?;

        if file.size() > limits.max_entry_size {
            return Err(OdtError::EntryTooLarge { name });
        }
        if name.starts_with("Pictures/") && file.size() > limits.max_image_size {
            return Err(OdtError::ImageTooLarge { name });
        }

        expanded_size = expanded_size.saturating_add(file.size());
        if expanded_size > limits.max_total_expanded_size {
            return Err(OdtError::ExpandedSizeTooLarge);
        }

        match name.as_str() {
            "content.xml" => {
                has_content = true;
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                validate_xml_preflight("content.xml", &content, limits)?;
            }
            "meta.xml" | "styles.xml" | "META-INF/manifest.xml" => {
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                validate_xml_preflight(&name, &content, limits)?;
            }
            "mimetype" => {
                if index != 0 || file.compression() != CompressionMethod::Stored {
                    return Err(OdtError::InvalidMimeType);
                }
                let mut value = String::new();
                file.read_to_string(&mut value)?;
                if value != ODT_MIME_TYPE {
                    return Err(OdtError::InvalidMimeType);
                }
                has_valid_mimetype = true;
            }
            _ if name.starts_with("Pictures/") => {
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)?;
                if detect_image_media_type(&bytes).is_none() {
                    return Err(OdtError::UnsupportedImageType { name });
                }
            }
            _ => {}
        }
    }

    if !has_content {
        return Err(OdtError::MissingContent);
    }

    if !has_valid_mimetype {
        return Err(OdtError::InvalidMimeType);
    }

    Ok(())
}

fn validate_entry_path(name: &str, limits: PackageLimits) -> Result<(), OdtError> {
    if name.starts_with('/')
        || name.starts_with('\\')
        || name.contains('\\')
        || name.split('/').any(|part| part == ".." || part.is_empty())
    {
        return Err(OdtError::UnsafePath {
            name: name.to_string(),
        });
    }
    if name.split('/').count() > limits.max_path_depth {
        return Err(OdtError::PathTooDeep {
            name: name.to_string(),
        });
    }
    Ok(())
}

fn validate_entry_mode(file: &zip::read::ZipFile<'_>, name: &str) -> Result<(), OdtError> {
    const UNIX_FILE_TYPE_MASK: u32 = 0o170000;
    const UNIX_SYMLINK: u32 = 0o120000;

    if let Some(mode) = file.unix_mode() {
        if mode & UNIX_FILE_TYPE_MASK == UNIX_SYMLINK {
            return Err(OdtError::SymlinkEntry {
                name: name.to_string(),
            });
        }
    }
    if file.encrypted() {
        return Err(OdtError::EncryptedEntry {
            name: name.to_string(),
        });
    }
    Ok(())
}

fn validate_entry_kind(name: &str) -> Result<(), OdtError> {
    let lower = name.to_ascii_lowercase();
    let executable = lower.starts_with("scripts/")
        || lower.starts_with("basic/")
        || lower.ends_with(".exe")
        || lower.ends_with(".dll")
        || lower.ends_with(".dylib")
        || lower.ends_with(".so")
        || lower.ends_with(".js")
        || lower.ends_with(".sh")
        || lower.ends_with(".bat")
        || lower.ends_with(".cmd");

    if executable {
        return Err(OdtError::ExecutableEntry {
            name: name.to_string(),
        });
    }
    Ok(())
}

fn validate_xml_preflight(
    name: &str,
    content: &str,
    limits: PackageLimits,
) -> Result<(), OdtError> {
    let lower = content.to_ascii_lowercase();
    if lower.contains("<!doctype") || lower.contains("<!entity") {
        return Err(OdtError::XmlEntityDeclaration {
            name: name.to_string(),
        });
    }

    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(false);
    let mut depth = 0_usize;
    loop {
        match reader.read_event().map_err(|err| xml_error(name, err))? {
            Event::Start(_) => {
                depth += 1;
                if depth > limits.max_xml_depth {
                    return Err(OdtError::XmlTooDeep {
                        name: name.to_string(),
                    });
                }
            }
            Event::Empty(_) => {
                if depth + 1 > limits.max_xml_depth {
                    return Err(OdtError::XmlTooDeep {
                        name: name.to_string(),
                    });
                }
            }
            Event::End(_) => depth = depth.saturating_sub(1),
            Event::DocType(_) => {
                return Err(OdtError::XmlEntityDeclaration {
                    name: name.to_string(),
                })
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(())
}

fn render_content_xml(document: &Document) -> Result<String, OdtError> {
    let mut body = String::new();
    for section in &document.sections {
        for block in &section.blocks {
            render_block(block, document, &mut body)?;
        }
    }

    let automatic_styles = render_automatic_styles(document);
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <office:document-content \
         xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" \
         xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" \
         xmlns:style=\"urn:oasis:names:tc:opendocument:xmlns:style:1.0\" \
         xmlns:fo=\"urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0\" \
         xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\" \
         xmlns:draw=\"urn:oasis:names:tc:opendocument:xmlns:drawing:1.0\" \
         xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
         xmlns:svg=\"urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0\" \
         xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
         xmlns:word900=\"urn:900labs:900word:metadata\" \
         office:version=\"1.3\">\
         {automatic_styles}<office:body><office:text word900:track-changes-recording=\"{}\">{body}</office:text></office:body>\
         </office:document-content>",
        document.track_changes.recording
    ))
}

fn render_automatic_styles(document: &Document) -> String {
    let mut output = String::from("<office:automatic-styles>");
    let default_page = PageSetup::default();
    let page = document
        .sections
        .first()
        .map(|section| &section.page)
        .unwrap_or(&default_page);
    output.push_str(&render_page_layout_style(page));
    output.push_str(
        "<text:list-style style:name=\"900w-unordered\">\
         <text:list-level-style-bullet text:level=\"1\" text:bullet-char=\"&#8226;\"/>\
         </text:list-style>\
         <text:list-style style:name=\"900w-ordered\">\
         <text:list-level-style-number text:level=\"1\" style:num-format=\"1\"/>\
         </text:list-style>\
         <style:style style:name=\"900w-image\" style:family=\"paragraph\"/>\
         <style:style style:name=\"900w-toc\" style:family=\"paragraph\"/>",
    );

    for style in document.styles.values() {
        if style.kind == StyleKind::Paragraph && safe_style_name(style.id.as_str()) {
            output.push_str(&render_paragraph_style(
                style.id.as_str(),
                Some(&style.name),
                style.properties.paragraph.as_ref(),
            ));
        }
    }

    for (style_name, format) in collect_paragraph_direct_styles(document) {
        output.push_str(&render_paragraph_style(&style_name, None, Some(&format)));
    }

    for name in collect_text_style_names(document) {
        output.push_str(&render_text_style(&name));
    }

    output.push_str("</office:automatic-styles>");
    output
}

fn render_paragraph_style(
    name: &str,
    display_name: Option<&str>,
    format: Option<&ParagraphFormat>,
) -> String {
    let display = display_name
        .map(|value| format!(" style:display-name=\"{}\"", escape_xml(value)))
        .unwrap_or_default();
    let properties = format
        .map(render_paragraph_properties)
        .unwrap_or_else(|| "<style:paragraph-properties/>".to_string());
    format!(
        "<style:style style:name=\"{}\"{display} style:family=\"paragraph\">{properties}</style:style>",
        escape_xml(name)
    )
}

fn render_paragraph_properties(format: &ParagraphFormat) -> String {
    let mut attrs = String::new();
    if let Some(alignment) = format.alignment {
        let value = match alignment {
            ParagraphAlignment::Left => "left",
            ParagraphAlignment::Center => "center",
            ParagraphAlignment::Right => "right",
            ParagraphAlignment::Justify => "justify",
        };
        attrs.push_str(&format!(" fo:text-align=\"{value}\""));
    }
    if let Some(line_spacing) = format.line_spacing_per_mille {
        attrs.push_str(&format!(" fo:line-height=\"{}%\"", line_spacing / 10));
    }
    if let Some(spacing_before) = format.spacing_before_mm {
        attrs.push_str(&format!(" fo:margin-top=\"{spacing_before}mm\""));
    }
    if let Some(spacing_after) = format.spacing_after_mm {
        attrs.push_str(&format!(" fo:margin-bottom=\"{spacing_after}mm\""));
    }
    if let Some(indent_start) = format.indent_start_mm {
        attrs.push_str(&format!(" fo:margin-left=\"{indent_start}mm\""));
    }
    if let Some(indent_end) = format.indent_end_mm {
        attrs.push_str(&format!(" fo:margin-right=\"{indent_end}mm\""));
    }
    if let Some(first_line_indent) = format.first_line_indent_mm {
        attrs.push_str(&format!(" fo:text-indent=\"{first_line_indent}mm\""));
    }
    format!("<style:paragraph-properties{attrs}/>")
}

fn render_page_layout_style(page: &PageSetup) -> String {
    format!(
        "<style:page-layout style:name=\"{PAGE_LAYOUT_STYLE}\">\
         <style:page-layout-properties \
         fo:page-width=\"{}mm\" \
         fo:page-height=\"{}mm\" \
         fo:margin-top=\"{}mm\" \
         fo:margin-right=\"{}mm\" \
         fo:margin-bottom=\"{}mm\" \
         fo:margin-left=\"{}mm\"/>\
         </style:page-layout>",
        page.width_mm,
        page.height_mm,
        page.margin_top_mm,
        page.margin_right_mm,
        page.margin_bottom_mm,
        page.margin_left_mm
    )
}

fn render_text_style(name: &str) -> String {
    let marks = marks_from_text_style(name);
    let inline_style = inline_style_from_text_style(name);
    let mut properties = String::new();
    if marks.contains(&InlineMark::Bold) {
        properties.push_str(" fo:font-weight=\"bold\"");
    }
    if marks.contains(&InlineMark::Italic) {
        properties.push_str(" fo:font-style=\"italic\"");
    }
    if marks.contains(&InlineMark::Underline) {
        properties
            .push_str(" style:text-underline-style=\"solid\" style:text-underline-type=\"single\"");
    }
    if marks.contains(&InlineMark::Strikethrough) {
        properties.push_str(" style:text-line-through-style=\"solid\"");
    }
    if marks.contains(&InlineMark::Superscript) {
        properties.push_str(" style:text-position=\"super 58%\"");
    }
    if marks.contains(&InlineMark::Subscript) {
        properties.push_str(" style:text-position=\"sub 58%\"");
    }
    if let Some(font_family) = inline_style.font_family.as_deref() {
        properties.push_str(&format!(" fo:font-family=\"{}\"", escape_xml(font_family)));
    }
    if let Some(font_size) = inline_style.font_size_pt {
        properties.push_str(&format!(" fo:font-size=\"{font_size}pt\""));
    }
    if let Some(text_color) = inline_style.text_color.as_deref() {
        properties.push_str(&format!(" fo:color=\"{}\"", escape_xml(text_color)));
    }
    if let Some(highlight_color) = inline_style.highlight_color.as_deref() {
        properties.push_str(&format!(
            " fo:background-color=\"{}\"",
            escape_xml(highlight_color)
        ));
    }

    format!(
        "<style:style style:name=\"{}\" style:family=\"text\"><style:text-properties{properties}/></style:style>",
        escape_xml(name)
    )
}

fn render_block(block: &Block, document: &Document, output: &mut String) -> Result<(), OdtError> {
    match block {
        Block::Paragraph(paragraph) => {
            let style_name = if paragraph.format.is_default() {
                paragraph.style.as_str().to_string()
            } else {
                paragraph_style_name(paragraph.style.as_str(), &paragraph.format)
            };
            output.push_str(&format!(
                "<text:p text:style-name=\"{}\">",
                escape_xml(&style_name),
            ));
            render_bookmark(paragraph.bookmark_id.as_deref(), output);
            render_inlines(&paragraph.inlines, Some(document), output);
            output.push_str("</text:p>");
        }
        Block::Heading(heading) => {
            output.push_str(&format!(
                "<text:h text:outline-level=\"{}\">",
                heading.level.clamp(1, 6),
            ));
            render_bookmark(heading.bookmark_id.as_deref(), output);
            render_inlines(&heading.inlines, Some(document), output);
            output.push_str("</text:h>");
        }
        Block::TableOfContents(table_of_contents) => {
            render_table_of_contents(table_of_contents, output);
        }
        Block::List(list) => render_list(list, document, output)?,
        Block::Table(table) => render_table(table, document, output)?,
        Block::Image(image) => render_image(image, document, output)?,
        Block::PageBreak => {
            output.push_str("<text:soft-page-break/>");
        }
    }
    Ok(())
}

fn render_table_of_contents(table_of_contents: &TableOfContents, output: &mut String) {
    let entries_json =
        serde_json::to_string(&table_of_contents.entries).unwrap_or_else(|_| "[]".to_string());
    let title = if table_of_contents.title.trim().is_empty() {
        "Contents"
    } else {
        table_of_contents.title.trim()
    };
    output.push_str(&format!(
        "<text:p text:style-name=\"{TOC_PARAGRAPH_STYLE}\" word900:block-type=\"table-of-contents\" word900:toc-title=\"{}\" word900:toc-entries=\"{}\">",
        escape_xml(title),
        escape_xml(&entries_json)
    ));
    output.push_str(&escape_xml(title));
    for entry in &table_of_contents.entries {
        let Some(target) = sanitize_bookmark_id(&entry.target_bookmark_id) else {
            continue;
        };
        output.push_str("<text:line-break/>");
        for _ in 1..entry.level.clamp(1, 3) {
            output.push_str("  ");
        }
        output.push_str(&format!(
            "<text:a xlink:href=\"#{}\">{}</text:a>",
            escape_xml(&target),
            escape_xml(&entry.text)
        ));
    }
    output.push_str("</text:p>");
}

fn render_list(list: &ListBlock, document: &Document, output: &mut String) -> Result<(), OdtError> {
    let ordered = document
        .lists
        .get(&list.definition_id)
        .map(|definition| definition.ordered)
        .unwrap_or(false);
    let style_name = if ordered {
        ORDERED_LIST_STYLE
    } else {
        UNORDERED_LIST_STYLE
    };

    output.push_str(&format!(
        "<text:list text:style-name=\"{}\">",
        escape_xml(style_name)
    ));
    for item in &list.items {
        output.push_str(&format!(
            "<text:list-item text:level=\"{}\">",
            item.level.clamp(1, 8)
        ));
        for block in &item.blocks {
            render_block(block, document, output)?;
        }
        output.push_str("</text:list-item>");
    }
    output.push_str("</text:list>");
    Ok(())
}

fn render_table(table: &Table, document: &Document, output: &mut String) -> Result<(), OdtError> {
    output.push_str("<table:table>");
    for row in &table.rows {
        output.push_str("<table:table-row>");
        for cell in &row.cells {
            output.push_str("<table:table-cell");
            output.push_str(&table_cell_presentation_attrs(&cell.presentation));
            output.push('>');
            for block in &cell.blocks {
                render_block(block, document, output)?;
            }
            output.push_str("</table:table-cell>");
        }
        output.push_str("</table:table-row>");
    }
    output.push_str("</table:table>");
    Ok(())
}

fn table_cell_presentation_attrs(presentation: &TableCellPresentation) -> String {
    if presentation.is_default() {
        return String::new();
    }

    let mut attrs = String::new();
    if let Some(background_color) = presentation
        .background_color
        .as_deref()
        .and_then(sanitize_table_cell_background_color)
    {
        attrs.push_str(" word900:cell-background-color=\"");
        attrs.push_str(&escape_xml(&background_color));
        attrs.push('"');
    }
    if let Some(alignment) = presentation.text_alignment {
        attrs.push_str(" word900:cell-text-align=\"");
        attrs.push_str(paragraph_alignment_name(alignment));
        attrs.push('"');
    }
    if presentation.border != TableCellBorder::Visible {
        attrs.push_str(" word900:cell-border=\"");
        attrs.push_str(table_cell_border_name(presentation.border));
        attrs.push('"');
    }
    attrs
}

fn paragraph_alignment_name(alignment: ParagraphAlignment) -> &'static str {
    match alignment {
        ParagraphAlignment::Left => "left",
        ParagraphAlignment::Center => "center",
        ParagraphAlignment::Right => "right",
        ParagraphAlignment::Justify => "justify",
    }
}

fn table_cell_border_name(border: TableCellBorder) -> &'static str {
    match border {
        TableCellBorder::Visible => "visible",
        TableCellBorder::Hidden => "hidden",
    }
}

fn render_image(
    image: &ImageBlock,
    document: &Document,
    output: &mut String,
) -> Result<(), OdtError> {
    let asset = document
        .assets
        .get(&image.asset_id)
        .ok_or_else(|| OdtError::MissingAsset {
            asset_id: image.asset_id.clone(),
        })?;
    let href = asset_package_path(asset)?;
    let alt = image.alt_text.as_deref().unwrap_or_default();
    let caption_attr = image
        .presentation
        .caption
        .as_deref()
        .map(|caption| format!(" word900:caption=\"{}\"", escape_xml(caption)))
        .unwrap_or_default();
    let scale = image.presentation.scale_percent.clamp(25, 200);
    output.push_str(&format!(
        "<text:p text:style-name=\"{IMAGE_PARAGRAPH_STYLE}\">\
         <draw:frame draw:name=\"{}\" svg:title=\"{}\" word900:alignment=\"{}\" word900:scale-percent=\"{}\"{}>\
         <draw:image xlink:href=\"{}\" xlink:type=\"simple\" xlink:show=\"embed\" xlink:actuate=\"onLoad\"/>\
         </draw:frame></text:p>",
        escape_xml(&image.asset_id),
        escape_xml(alt),
        image_alignment_name(image.presentation.alignment),
        scale,
        caption_attr,
        escape_xml(&href)
    ));
    Ok(())
}

fn render_bookmark(bookmark_id: Option<&str>, output: &mut String) {
    if let Some(id) = bookmark_id.and_then(sanitize_bookmark_id) {
        output.push_str("<text:bookmark text:name=\"");
        output.push_str(&escape_xml(&id));
        output.push_str("\"/>");
    }
}

fn image_alignment_name(alignment: ImageAlignment) -> &'static str {
    match alignment {
        ImageAlignment::Inline => "inline",
        ImageAlignment::Left => "left",
        ImageAlignment::Center => "center",
        ImageAlignment::Right => "right",
    }
}

fn render_inlines(inlines: &[Inline], document: Option<&Document>, output: &mut String) {
    let mut active_comments: Vec<String> = Vec::new();
    for inline in inlines {
        let next_comments = inline_comment_ids(inline, document);
        close_inactive_comments(&mut active_comments, &next_comments, output);
        open_new_comments(&mut active_comments, &next_comments, document, output);

        if let Some(reference) = inline.note_reference.as_ref() {
            output.push_str(&render_note(reference, document));
            continue;
        }

        if let Some(field) = inline.field {
            output.push_str(&render_page_field(field, &inline.text));
            continue;
        }
        let mut rendered = escape_xml(&inline.text);
        let style_name = text_style_name(&inline.marks, &inline.style);
        let span_attrs = inline_span_attrs(style_name.as_deref(), inline.tracked_change.as_ref());
        if !span_attrs.is_empty() {
            rendered = format!("<text:span{span_attrs}>{rendered}</text:span>");
        }
        if let Some(href) = inline.link.as_deref().and_then(sanitize_text_href) {
            rendered = format!(
                "<text:a xlink:href=\"{}\">{rendered}</text:a>",
                escape_xml(&href)
            );
        }
        output.push_str(&rendered);
    }
    close_inactive_comments(&mut active_comments, &[], output);
}

fn render_note(reference: &InlineNoteReference, document: Option<&Document>) -> String {
    let Ok(reference) = validate_note_reference(reference) else {
        return escape_xml(&reference.label);
    };
    let Some(note) = document.and_then(|document| document.notes.get(&reference.id)) else {
        return escape_xml(&reference.label);
    };
    if note.kind != reference.kind {
        return escape_xml(&reference.label);
    }
    format!(
        "<text:note text:id=\"{}\" text:note-class=\"{}\" word900:note-id=\"{}\" word900:note-kind=\"{}\">\
         <text:note-citation>{}</text:note-citation>\
         <text:note-body><text:p>{}</text:p></text:note-body>\
         </text:note>",
        escape_xml(&reference.id),
        note_kind_name(reference.kind),
        escape_xml(&reference.id),
        note_kind_name(reference.kind),
        escape_xml(&reference.label),
        escape_xml(&note.body)
    )
}

fn note_kind_name(kind: NoteKind) -> &'static str {
    match kind {
        NoteKind::Footnote => "footnote",
        NoteKind::Endnote => "endnote",
    }
}

fn parse_note_kind(value: &str) -> Option<NoteKind> {
    match value {
        "footnote" => Some(NoteKind::Footnote),
        "endnote" => Some(NoteKind::Endnote),
        _ => None,
    }
}

fn inline_span_attrs(style_name: Option<&str>, tracked_change: Option<&TrackedChange>) -> String {
    let mut attrs = String::new();
    if let Some(style_name) = style_name {
        attrs.push_str(&format!(" text:style-name=\"{}\"", escape_xml(style_name)));
    }
    if let Some(change) =
        tracked_change.filter(|change| validate_tracked_change_id(&change.id).is_ok())
    {
        attrs.push_str(&format!(
            " word900:change-id=\"{}\" word900:change-kind=\"{}\" word900:change-author=\"{}\" word900:change-created-at=\"{}\"",
            escape_xml(&change.id),
            tracked_change_kind_name(change.kind),
            escape_xml(&change.author),
            escape_xml(&change.created_at.to_rfc3339())
        ));
    }
    attrs
}

fn tracked_change_kind_name(kind: TrackedChangeKind) -> &'static str {
    match kind {
        TrackedChangeKind::Insertion => "insertion",
        TrackedChangeKind::Deletion => "deletion",
    }
}

fn parse_tracked_change_kind(value: &str) -> Option<TrackedChangeKind> {
    match value {
        "insertion" => Some(TrackedChangeKind::Insertion),
        "deletion" => Some(TrackedChangeKind::Deletion),
        _ => None,
    }
}

fn inline_comment_ids(inline: &Inline, document: Option<&Document>) -> Vec<String> {
    let Some(document) = document else {
        return Vec::new();
    };
    let mut ids = Vec::new();
    for id in &inline.comment_ids {
        if ids.contains(id) {
            continue;
        }
        if validate_comment_id(id).is_ok() && document.comments.contains_key(id) {
            ids.push(id.clone());
        }
    }
    ids
}

fn close_inactive_comments(active: &mut Vec<String>, next: &[String], output: &mut String) {
    let common_prefix = active
        .iter()
        .zip(next.iter())
        .take_while(|(left, right)| left == right)
        .count();
    while active.len() > common_prefix {
        let id = active.pop().expect("checked active comment");
        output.push_str(&format!(
            "<office:annotation-end office:name=\"{}\"/>",
            escape_xml(&id)
        ));
    }
}

fn open_new_comments(
    active: &mut Vec<String>,
    next: &[String],
    document: Option<&Document>,
    output: &mut String,
) {
    let Some(document) = document else {
        return;
    };
    for id in next {
        if active.contains(id) {
            continue;
        }
        if let Some(comment) = document.comments.get(id) {
            output.push_str(&render_annotation_start(comment));
            active.push(id.clone());
        }
    }
}

fn render_annotation_start(comment: &CommentThread) -> String {
    let resolved = if comment.resolved { "true" } else { "false" };
    format!(
        "<office:annotation office:name=\"{}\" word900:comment-id=\"{}\" word900:resolved=\"{}\">\
         <dc:creator>{}</dc:creator><dc:date>{}</dc:date><text:p>{}</text:p>\
         </office:annotation>",
        escape_xml(&comment.id),
        escape_xml(&comment.id),
        resolved,
        escape_xml(&comment.author),
        escape_xml(&comment.created_at.to_rfc3339()),
        escape_xml(&comment.body)
    )
}

fn render_page_field(field: PageField, fallback: &str) -> String {
    let text = if fallback.is_empty() {
        field.fallback_text()
    } else {
        fallback
    };
    match field {
        PageField::PageNumber => {
            format!("<text:page-number>{}</text:page-number>", escape_xml(text))
        }
        PageField::PageCount => format!("<text:page-count>{}</text:page-count>", escape_xml(text)),
        PageField::Date => format!("<text:date>{}</text:date>", escape_xml(text)),
    }
}

fn render_meta_xml(document: &Document) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <office:document-meta xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" \
         xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
         xmlns:meta=\"urn:oasis:names:tc:opendocument:xmlns:meta:1.0\" office:version=\"1.3\">\
         <office:meta><dc:title>{}</dc:title><meta:generator>900Word</meta:generator></office:meta>\
         </office:document-meta>",
        escape_xml(&document.meta.title)
    )
}

fn render_styles_xml(document: &Document) -> String {
    let default_page = PageSetup::default();
    let page = document
        .sections
        .first()
        .map(|section| &section.page)
        .unwrap_or(&default_page);
    let mut output = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <office:document-styles \
         xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" \
         xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" \
         xmlns:style=\"urn:oasis:names:tc:opendocument:xmlns:style:1.0\" \
         xmlns:fo=\"urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0\" \
         xmlns:word900=\"urn:900labs:900word:metadata\" \
         office:version=\"1.3\">\
         <office:automatic-styles>{}</office:automatic-styles>",
        render_page_layout_style(page)
    );

    output.push_str(&format!(
        "<office:master-styles><style:master-page style:name=\"{MASTER_PAGE_STYLE}\" style:page-layout-name=\"{PAGE_LAYOUT_STYLE}\" word900:different-first-page=\"{}\">",
        document
            .sections
            .first()
            .map(|section| section.page_regions.different_first_page)
            .unwrap_or(false)
    ));

    if let Some(section) = document.sections.first() {
        render_page_region("style:header", &section.page_regions.header, &mut output);
        render_page_region("style:footer", &section.page_regions.footer, &mut output);
        render_page_region(
            "style:header-first",
            &section.page_regions.first_header,
            &mut output,
        );
        render_page_region(
            "style:footer-first",
            &section.page_regions.first_footer,
            &mut output,
        );
    }

    output.push_str("</style:master-page></office:master-styles></office:document-styles>");
    output
}

fn render_page_region(tag: &str, region: &PageRegion, output: &mut String) {
    if region.blocks.is_empty() {
        return;
    }
    output.push('<');
    output.push_str(tag);
    output.push('>');
    for block in &region.blocks {
        render_page_region_block(block, output);
    }
    output.push_str("</");
    output.push_str(tag);
    output.push('>');
}

fn render_page_region_block(block: &PageRegionBlock, output: &mut String) {
    match block {
        PageRegionBlock::Paragraph(paragraph) => {
            output.push_str("<text:p>");
            render_inlines(&paragraph.inlines, None, output);
            output.push_str("</text:p>");
        }
    }
}

fn render_manifest_xml(document: &Document) -> Result<String, OdtError> {
    let mut output = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\" manifest:version=\"1.3\">\
         <manifest:file-entry manifest:full-path=\"/\" manifest:media-type=\"{ODT_MIME_TYPE}\"/>\
         <manifest:file-entry manifest:full-path=\"content.xml\" manifest:media-type=\"text/xml\"/>\
         <manifest:file-entry manifest:full-path=\"meta.xml\" manifest:media-type=\"text/xml\"/>\
         <manifest:file-entry manifest:full-path=\"styles.xml\" manifest:media-type=\"text/xml\"/>"
    );

    for asset in image_assets_in_document(document)? {
        let media_type = validate_image_asset(asset)?;
        output.push_str(&format!(
            "<manifest:file-entry manifest:full-path=\"{}\" manifest:media-type=\"{}\"/>",
            escape_xml(&asset_package_path(asset)?),
            escape_xml(media_type)
        ));
    }

    output.push_str("</manifest:manifest>");
    Ok(output)
}

fn image_assets_in_document(document: &Document) -> Result<Vec<&AssetRef>, OdtError> {
    let mut ids = BTreeSet::new();
    for section in &document.sections {
        collect_image_asset_ids_from_blocks(&section.blocks, &mut ids);
    }

    ids.into_iter()
        .map(|asset_id| {
            document
                .assets
                .get(&asset_id)
                .ok_or(OdtError::MissingAsset { asset_id })
        })
        .collect()
}

fn collect_image_asset_ids_from_blocks(blocks: &[Block], ids: &mut BTreeSet<String>) {
    for block in blocks {
        match block {
            Block::Image(image) => {
                ids.insert(image.asset_id.clone());
            }
            Block::List(list) => {
                for item in &list.items {
                    collect_image_asset_ids_from_blocks(&item.blocks, ids);
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_image_asset_ids_from_blocks(&cell.blocks, ids);
                    }
                }
            }
            _ => {}
        }
    }
}

fn asset_package_path(asset: &AssetRef) -> Result<String, OdtError> {
    if !safe_asset_name(&asset.id) {
        return Err(OdtError::UnsafeAssetName {
            asset_id: asset.id.clone(),
        });
    }
    Ok(format!("Pictures/{}", asset.id))
}

fn validate_image_asset(asset: &AssetRef) -> Result<&'static str, OdtError> {
    let detected =
        detect_image_media_type(&asset.bytes).ok_or_else(|| OdtError::UnsupportedImageType {
            name: asset.id.clone(),
        })?;
    if asset.media_type != detected || asset.byte_len != asset.bytes.len() {
        return Err(OdtError::UnsupportedImageType {
            name: asset.id.clone(),
        });
    }
    Ok(detected)
}

fn safe_asset_name(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains("..")
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | '@'))
}

fn safe_style_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
}

fn collect_text_style_names(document: &Document) -> BTreeSet<String> {
    let mut styles = BTreeSet::new();
    for section in &document.sections {
        collect_text_styles_from_blocks(&section.blocks, &mut styles);
    }
    styles
}

fn collect_paragraph_direct_styles(document: &Document) -> BTreeMap<String, ParagraphFormat> {
    let mut styles = BTreeMap::new();
    for section in &document.sections {
        collect_paragraph_styles_from_blocks(&section.blocks, &mut styles);
    }
    styles
}

fn collect_paragraph_styles_from_blocks(
    blocks: &[Block],
    styles: &mut BTreeMap<String, ParagraphFormat>,
) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                if !paragraph.format.is_default() {
                    styles.insert(
                        paragraph_style_name(paragraph.style.as_str(), &paragraph.format),
                        paragraph.format.clone(),
                    );
                }
            }
            Block::List(list) => {
                for item in &list.items {
                    collect_paragraph_styles_from_blocks(&item.blocks, styles);
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_paragraph_styles_from_blocks(&cell.blocks, styles);
                    }
                }
            }
            _ => {}
        }
    }
}

fn collect_text_styles_from_blocks(blocks: &[Block], styles: &mut BTreeSet<String>) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                collect_text_styles_from_inlines(&paragraph.inlines, styles)
            }
            Block::Heading(heading) => collect_text_styles_from_inlines(&heading.inlines, styles),
            Block::List(list) => {
                for item in &list.items {
                    collect_text_styles_from_blocks(&item.blocks, styles);
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_text_styles_from_blocks(&cell.blocks, styles);
                    }
                }
            }
            _ => {}
        }
    }
}

fn collect_text_styles_from_inlines(inlines: &[Inline], styles: &mut BTreeSet<String>) {
    for inline in inlines {
        if let Some(style_name) = text_style_name(&inline.marks, &inline.style) {
            styles.insert(style_name);
        }
    }
}

fn parse_content_xml(
    content: &str,
    asset_payloads: &BTreeMap<String, AssetPayload>,
) -> Result<Document, OdtError> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(false);

    let mut state = ParseState::new(asset_payloads);
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error("content.xml", err))?
        {
            Event::Start(start) => state.start(&start)?,
            Event::Empty(start) => state.empty(&start)?,
            Event::End(end) => state.end(end.name().as_ref())?,
            Event::Text(text) => {
                state.text(
                    &text
                        .xml10_content()
                        .map_err(|err| xml_error("content.xml", err))?,
                );
            }
            Event::CData(text) => {
                state.text(
                    &text
                        .xml10_content()
                        .map_err(|err| xml_error("content.xml", err))?,
                );
            }
            Event::DocType(_) => {
                return Err(OdtError::XmlEntityDeclaration {
                    name: "content.xml".to_string(),
                })
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(state.into_document())
}

fn prune_unanchored_imported_comments(document: &mut Document) {
    if document.comments.is_empty() {
        return;
    }
    let mut anchored = BTreeSet::new();
    for section in &document.sections {
        collect_imported_comment_anchor_ids_from_blocks(&section.blocks, &mut anchored);
    }
    document.comments.retain(|id, _| anchored.contains(id));
}

fn collect_imported_comment_anchor_ids_from_blocks(blocks: &[Block], ids: &mut BTreeSet<String>) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                collect_imported_comment_anchor_ids_from_inlines(&paragraph.inlines, ids);
            }
            Block::Heading(heading) => {
                collect_imported_comment_anchor_ids_from_inlines(&heading.inlines, ids);
            }
            Block::List(list) => {
                for item in &list.items {
                    collect_imported_comment_anchor_ids_from_blocks(&item.blocks, ids);
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_imported_comment_anchor_ids_from_blocks(&cell.blocks, ids);
                    }
                }
            }
            Block::TableOfContents(_) | Block::Image(_) | Block::PageBreak => {}
        }
    }
}

fn collect_imported_comment_anchor_ids_from_inlines(
    inlines: &[Inline],
    ids: &mut BTreeSet<String>,
) {
    for inline in inlines {
        for id in &inline.comment_ids {
            if validate_comment_id(id).is_ok() {
                ids.insert(id.clone());
            }
        }
    }
}

#[derive(Debug, Default)]
struct ParsedPageRegions {
    regions: PageRegions,
    warnings: Vec<DocumentWarning>,
}

fn parse_page_regions_xml(styles: &str) -> Result<ParsedPageRegions, OdtError> {
    let mut reader = Reader::from_str(styles);
    reader.config_mut().trim_text(false);

    let mut state = PageRegionParseState::default();
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error("styles.xml", err))?
        {
            Event::Start(start) => state.start(&start)?,
            Event::Empty(start) => state.empty(&start)?,
            Event::End(end) => state.end(end.name().as_ref()),
            Event::Text(text) => {
                if state.unsupported_depth == 0 {
                    if let Some(active) = state.active_text.as_mut() {
                        active.text.push_str(
                            &text
                                .xml10_content()
                                .map_err(|err| xml_error("styles.xml", err))?,
                        );
                    }
                }
            }
            Event::CData(text) => {
                if state.unsupported_depth == 0 {
                    if let Some(active) = state.active_text.as_mut() {
                        active.text.push_str(
                            &text
                                .xml10_content()
                                .map_err(|err| xml_error("styles.xml", err))?,
                        );
                    }
                }
            }
            Event::DocType(_) => {
                return Err(OdtError::XmlEntityDeclaration {
                    name: "styles.xml".to_string(),
                })
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(ParsedPageRegions {
        regions: state.regions,
        warnings: state.warnings,
    })
}

#[derive(Debug, Default)]
struct PageRegionParseState {
    regions: PageRegions,
    warnings: Vec<DocumentWarning>,
    active_region: Option<PageRegionKind>,
    active_text: Option<ActiveText>,
    unsupported_depth: usize,
    warned_regions: BTreeSet<&'static str>,
}

impl PageRegionParseState {
    fn start(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        if self.unsupported_depth > 0 {
            self.unsupported_depth += 1;
            return Ok(());
        }

        match local_name(start.name().as_ref()) {
            b"master-page" => {
                if attr_value(start, b"different-first-page")?.as_deref() == Some("true") {
                    self.regions.different_first_page = true;
                }
            }
            b"header" => self.active_region = Some(PageRegionKind::Header),
            b"footer" => self.active_region = Some(PageRegionKind::Footer),
            b"header-first" => self.active_region = Some(PageRegionKind::FirstHeader),
            b"footer-first" => self.active_region = Some(PageRegionKind::FirstFooter),
            b"p" if self.active_region.is_some() => {
                self.active_text = Some(ActiveText::paragraph(
                    StyleId::from("body"),
                    ParagraphFormat::default(),
                    None,
                ));
            }
            b"span" if self.active_text.is_some() => {
                if let Some(active) = self.active_text.as_mut() {
                    active.flush();
                    let style_name = attr_value(start, b"style-name")?;
                    if let Some(style_name) = style_name.as_deref() {
                        if !is_supported_generated_text_style(style_name) {
                            self.mark_active_region_read_only();
                            self.unsupported_depth = 1;
                            return Ok(());
                        }
                    }
                    let marks = style_name
                        .as_deref()
                        .map(marks_from_text_style)
                        .unwrap_or_default();
                    let inline_style = style_name
                        .as_deref()
                        .map(inline_style_from_text_style)
                        .unwrap_or_default();
                    active.mark_stack.push(marks);
                    active.style_stack.push(inline_style);
                }
            }
            b"a" if self.active_text.is_some() => {
                let href = attr_value(start, b"href")?;
                let sanitized = href.as_deref().and_then(sanitize_text_href);
                if href.is_some() && sanitized.is_none() {
                    self.warn(
                        "odt_header_footer_unsafe_link",
                        "Unsafe header/footer link was stripped during import",
                    );
                }
                if let Some(active) = self.active_text.as_mut() {
                    active.flush();
                    active.link_stack.push(sanitized);
                }
            }
            b"page-number" => self.start_page_field(PageField::PageNumber),
            b"page-count" => self.start_page_field(PageField::PageCount),
            b"date" => self.start_page_field(PageField::Date),
            b"document-styles"
            | b"automatic-styles"
            | b"master-styles"
            | b"style"
            | b"page-layout"
            | b"page-layout-properties" => {}
            _ if self.active_region.is_some() => {
                self.mark_active_region_read_only();
                self.unsupported_depth = 1;
            }
            _ => {}
        }
        Ok(())
    }

    fn empty(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        if self.unsupported_depth > 0 {
            return Ok(());
        }

        match local_name(start.name().as_ref()) {
            b"s" => {
                if let Some(active) = self.active_text.as_mut() {
                    let count = attr_value(start, b"c")?
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(1);
                    active.text.push_str(&" ".repeat(count.min(1000)));
                }
            }
            b"line-break" => {
                if let Some(active) = self.active_text.as_mut() {
                    active.text.push('\n');
                }
            }
            b"page-number" => self.push_page_field(PageField::PageNumber),
            b"page-count" => self.push_page_field(PageField::PageCount),
            b"date" => self.push_page_field(PageField::Date),
            b"header" | b"footer" | b"header-first" | b"footer-first" => {}
            b"document-styles"
            | b"automatic-styles"
            | b"master-styles"
            | b"style"
            | b"page-layout"
            | b"page-layout-properties" => {}
            _ if self.active_region.is_some() => self.mark_active_region_read_only(),
            _ => {}
        }
        Ok(())
    }

    fn end(&mut self, name: &[u8]) {
        if self.unsupported_depth > 0 {
            self.unsupported_depth -= 1;
            return;
        }

        match local_name(name) {
            b"p" => self.finish_paragraph(),
            b"span" => {
                if let Some(active) = self.active_text.as_mut() {
                    active.flush();
                    active.mark_stack.pop();
                    active.style_stack.pop();
                    active.tracked_change_stack.pop();
                }
            }
            b"a" => {
                if let Some(active) = self.active_text.as_mut() {
                    active.flush();
                    active.link_stack.pop();
                }
            }
            b"header" | b"footer" | b"header-first" | b"footer-first" => {
                self.active_region = None;
            }
            _ => {}
        }
    }

    fn start_page_field(&mut self, field: PageField) {
        self.push_page_field(field);
        self.unsupported_depth = 1;
    }

    fn push_page_field(&mut self, field: PageField) {
        if let Some(active) = self.active_text.as_mut() {
            active.push_field(field);
        }
    }

    fn finish_paragraph(&mut self) {
        let (Some(region), Some(mut active)) = (self.active_region, self.active_text.take()) else {
            return;
        };
        active.flush();
        self.regions
            .region_mut(region)
            .blocks
            .push(PageRegionBlock::Paragraph(PageRegionParagraph {
                inlines: active.inlines,
            }));
    }

    fn mark_active_region_read_only(&mut self) {
        let Some(region) = self.active_region else {
            return;
        };
        self.regions.region_mut(region).read_only = true;
        let label = page_region_label(region);
        if self.warned_regions.insert(label) {
            self.warn(
                "odt_header_footer_unsupported",
                "Unsupported header/footer content was imported as read-only",
            );
        }
    }

    fn warn(&mut self, code: &str, message: &str) {
        self.warnings.push(DocumentWarning {
            code: code.to_string(),
            message: message.to_string(),
        });
    }
}

fn page_region_label(region: PageRegionKind) -> &'static str {
    match region {
        PageRegionKind::Header => "header",
        PageRegionKind::Footer => "footer",
        PageRegionKind::FirstHeader => "first_header",
        PageRegionKind::FirstFooter => "first_footer",
    }
}

#[derive(Debug)]
struct ParseState<'a> {
    blocks: Vec<Block>,
    contexts: Vec<ParseContext>,
    active_text: Option<ActiveText>,
    active_frame: Option<ImageFrame>,
    active_style_name: Option<String>,
    styles: BTreeMap<StyleId, Style>,
    assets: BTreeMap<String, AssetRef>,
    asset_payloads: &'a BTreeMap<String, AssetPayload>,
    comments: BTreeMap<String, CommentThread>,
    lists: BTreeMap<String, ListDefinition>,
    page: PageSetup,
    track_changes: TrackChangesState,
    notes: BTreeMap<String, Note>,
    warnings: Vec<DocumentWarning>,
    unsupported_elements: BTreeSet<String>,
    unsupported_depth: usize,
    list_counter: usize,
    active_annotation: Option<AnnotationParse>,
    annotation_depth: usize,
    active_note: Option<NoteParse>,
    note_depth: usize,
}

impl<'a> ParseState<'a> {
    fn new(asset_payloads: &'a BTreeMap<String, AssetPayload>) -> Self {
        Self {
            blocks: Vec::new(),
            contexts: Vec::new(),
            active_text: None,
            active_frame: None,
            active_style_name: None,
            styles: Document::new_untitled().styles,
            assets: BTreeMap::new(),
            asset_payloads,
            comments: BTreeMap::new(),
            lists: Document::new_untitled().lists,
            page: PageSetup::default(),
            track_changes: TrackChangesState::default(),
            notes: BTreeMap::new(),
            warnings: Vec::new(),
            unsupported_elements: BTreeSet::new(),
            unsupported_depth: 0,
            list_counter: 0,
            active_annotation: None,
            annotation_depth: 0,
            active_note: None,
            note_depth: 0,
        }
    }

    fn start(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        if self.active_note.is_some() {
            self.note_depth += 1;
            if let Some(note) = self.active_note.as_mut() {
                note.start(local_name(start.name().as_ref()));
            }
            return Ok(());
        }

        if self.active_annotation.is_some() {
            self.annotation_depth += 1;
            if let Some(annotation) = self.active_annotation.as_mut() {
                annotation.start(local_name(start.name().as_ref()));
            }
            return Ok(());
        }

        if self.unsupported_depth > 0 {
            self.unsupported_depth += 1;
            return Ok(());
        }

        match local_name(start.name().as_ref()) {
            b"text" => self.read_track_changes_state(start)?,
            b"document-content" | b"automatic-styles" | b"body" => {}
            b"p" => self.start_paragraph(start)?,
            b"h" => self.start_heading(start)?,
            b"span" => self.start_span(start)?,
            b"a" => self.start_link(start)?,
            b"note" => self.start_note(start)?,
            b"annotation" => self.start_annotation(start)?,
            b"annotation-end" => self.end_annotation_anchor(start)?,
            b"page-number" => self.start_page_field(PageField::PageNumber),
            b"page-count" => self.start_page_field(PageField::PageCount),
            b"date" => self.start_page_field(PageField::Date),
            b"list" => self.start_list(start)?,
            b"list-item" => {
                let level = attr_value(start, b"level")?
                    .and_then(|value| value.parse::<u8>().ok())
                    .unwrap_or(1)
                    .clamp(1, 8);
                self.contexts.push(ParseContext::ListItem {
                    level,
                    blocks: Vec::new(),
                });
            }
            b"table" => self.contexts.push(ParseContext::Table { rows: Vec::new() }),
            b"table-row" => self
                .contexts
                .push(ParseContext::TableRow { cells: Vec::new() }),
            b"table-cell" => self.contexts.push(ParseContext::TableCell {
                presentation: parse_table_cell_presentation(start)?,
                blocks: Vec::new(),
            }),
            b"style" => self.start_style(start)?,
            b"page-layout" => {}
            b"page-layout-properties" => self.read_page_layout_properties(start)?,
            b"paragraph-properties" => self.read_paragraph_style_properties(start)?,
            b"text-properties"
            | b"list-style"
            | b"list-level-style-bullet"
            | b"list-level-style-number" => {}
            b"frame" => self.start_frame(start)?,
            b"image" => self.start_image(start)?,
            unknown => {
                self.warn_unsupported_element(unknown);
                self.unsupported_depth = 1;
            }
        }
        Ok(())
    }

    fn empty(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        if self.active_note.is_some() {
            if let Some(note) = self.active_note.as_mut() {
                note.empty(local_name(start.name().as_ref()));
            }
            return Ok(());
        }

        if self.active_annotation.is_some() {
            return Ok(());
        }

        if self.unsupported_depth > 0 {
            return Ok(());
        }

        match local_name(start.name().as_ref()) {
            b"s" => {
                if let Some(active) = self.active_text.as_mut() {
                    let count = attr_value(start, b"c")?
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(1);
                    active.text.push_str(&" ".repeat(count.min(1000)));
                }
            }
            b"line-break" => {
                if let Some(active) = self.active_text.as_mut() {
                    active.text.push('\n');
                }
            }
            b"bookmark" | b"bookmark-start" => self.read_inline_bookmark_id(start)?,
            b"bookmark-end" => {}
            b"note" => self.start_and_finish_empty_note(start)?,
            b"annotation" => self.start_and_finish_empty_annotation(start)?,
            b"annotation-end" => self.end_annotation_anchor(start)?,
            b"soft-page-break" => {
                if self.active_text.is_none() {
                    self.push_block(Block::PageBreak);
                } else {
                    self.warn(
                        "odt_inline_page_break",
                        "Inline page break was ignored during import",
                    );
                }
            }
            b"page-number" => self.push_page_field(PageField::PageNumber),
            b"page-count" => self.push_page_field(PageField::PageCount),
            b"date" => self.push_page_field(PageField::Date),
            b"image" => self.start_image(start)?,
            b"frame" => {
                self.start_frame(start)?;
                self.finish_frame();
            }
            b"style" => {
                self.start_style(start)?;
                self.active_style_name = None;
            }
            b"page-layout" => {}
            b"page-layout-properties" => self.read_page_layout_properties(start)?,
            b"document-content"
            | b"automatic-styles"
            | b"body"
            | b"text-properties"
            | b"list-style"
            | b"list-level-style-bullet"
            | b"list-level-style-number" => {}
            b"paragraph-properties" => self.read_paragraph_style_properties(start)?,
            b"text" => self.read_track_changes_state(start)?,
            unknown => self.warn_unsupported_element(unknown),
        }
        Ok(())
    }

    fn end(&mut self, name: &[u8]) -> Result<(), OdtError> {
        if self.active_note.is_some() {
            let local = local_name(name);
            if let Some(note) = self.active_note.as_mut() {
                note.end(local);
            }
            if local == b"note" && self.note_depth == 1 {
                self.finish_note();
            } else {
                self.note_depth = self.note_depth.saturating_sub(1);
            }
            return Ok(());
        }

        if self.active_annotation.is_some() {
            let local = local_name(name);
            if let Some(annotation) = self.active_annotation.as_mut() {
                annotation.end(local);
            }
            if local == b"annotation" && self.annotation_depth == 1 {
                self.finish_annotation_anchor();
            } else {
                self.annotation_depth = self.annotation_depth.saturating_sub(1);
            }
            return Ok(());
        }

        if self.unsupported_depth > 0 {
            self.unsupported_depth -= 1;
            return Ok(());
        }

        match local_name(name) {
            b"p" | b"h" => self.finish_text_block(),
            b"span" => {
                if let Some(active) = self.active_text.as_mut() {
                    active.flush();
                    active.mark_stack.pop();
                    active.style_stack.pop();
                }
            }
            b"a" => {
                if let Some(active) = self.active_text.as_mut() {
                    active.flush();
                    active.link_stack.pop();
                }
            }
            b"annotation-end" => {}
            b"list-item" => self.finish_list_item(),
            b"list" => self.finish_list(),
            b"table-cell" => self.finish_table_cell(),
            b"table-row" => self.finish_table_row(),
            b"table" => self.finish_table(),
            b"frame" => self.finish_frame(),
            b"style" => self.active_style_name = None,
            _ => {}
        }
        Ok(())
    }

    fn start_style(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        self.active_style_name = None;
        let Some(family) = attr_value(start, b"family")? else {
            return Ok(());
        };
        if family != "paragraph" {
            return Ok(());
        }

        let Some(style_name) = attr_value(start, b"name")? else {
            return Ok(());
        };
        if style_name == IMAGE_PARAGRAPH_STYLE {
            return Ok(());
        }
        if !safe_style_name(&style_name) {
            self.warn(
                "odt_unsafe_style_name",
                "Unsafe paragraph style name was ignored during import",
            );
            return Ok(());
        }

        let display_name =
            attr_value(start, b"display-name")?.unwrap_or_else(|| style_name.clone());
        self.active_style_name = Some(style_name.clone());
        self.styles.insert(
            StyleId::from(style_name.as_str()),
            Style {
                id: StyleId::from(style_name.as_str()),
                name: display_name,
                kind: StyleKind::Paragraph,
                parent: None,
                properties: Default::default(),
            },
        );
        Ok(())
    }

    fn read_paragraph_style_properties(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        let Some(style_name) = self.active_style_name.clone() else {
            return Ok(());
        };

        let mut format = self
            .styles
            .get(&StyleId::from(style_name.as_str()))
            .and_then(|style| style.properties.paragraph.clone())
            .unwrap_or_default();

        if let Some(value) = attr_value(start, b"text-align")? {
            format.alignment = match value.as_str() {
                "left" => Some(ParagraphAlignment::Left),
                "center" => Some(ParagraphAlignment::Center),
                "right" => Some(ParagraphAlignment::Right),
                "justify" => Some(ParagraphAlignment::Justify),
                _ => format.alignment,
            };
        }
        if let Some(value) = attr_value(start, b"line-height")? {
            if let Some(per_mille) = parse_line_height_per_mille(&value) {
                format.line_spacing_per_mille = Some(per_mille);
            }
        }
        if let Some(value) = attr_value(start, b"margin-top")? {
            if let Some(mm) = parse_mm_attr(&value) {
                format.spacing_before_mm = Some(mm);
            }
        }
        if let Some(value) = attr_value(start, b"margin-bottom")? {
            if let Some(mm) = parse_mm_attr(&value) {
                format.spacing_after_mm = Some(mm);
            }
        }
        if let Some(value) = attr_value(start, b"margin-left")? {
            if let Some(mm) = parse_mm_attr(&value) {
                format.indent_start_mm = Some(mm);
            }
        }
        if let Some(value) = attr_value(start, b"margin-right")? {
            if let Some(mm) = parse_mm_attr(&value) {
                format.indent_end_mm = Some(mm);
            }
        }
        if let Some(value) = attr_value(start, b"text-indent")? {
            if let Some(mm) = parse_signed_mm_attr(&value) {
                format.first_line_indent_mm = Some(mm);
            }
        }

        if let Some(style) = self.styles.get_mut(&StyleId::from(style_name.as_str())) {
            style.properties.paragraph = Some(format);
        }
        Ok(())
    }

    fn read_page_layout_properties(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        let mut page = self.page.clone();
        if let Some(value) = attr_value(start, b"page-width")? {
            if let Some(mm) = parse_mm_attr(&value) {
                page.width_mm = mm;
            }
        }
        if let Some(value) = attr_value(start, b"page-height")? {
            if let Some(mm) = parse_mm_attr(&value) {
                page.height_mm = mm;
            }
        }
        if let Some(value) = attr_value(start, b"margin-top")? {
            if let Some(mm) = parse_mm_attr(&value) {
                page.margin_top_mm = mm;
            }
        }
        if let Some(value) = attr_value(start, b"margin-right")? {
            if let Some(mm) = parse_mm_attr(&value) {
                page.margin_right_mm = mm;
            }
        }
        if let Some(value) = attr_value(start, b"margin-bottom")? {
            if let Some(mm) = parse_mm_attr(&value) {
                page.margin_bottom_mm = mm;
            }
        }
        if let Some(value) = attr_value(start, b"margin-left")? {
            if let Some(mm) = parse_mm_attr(&value) {
                page.margin_left_mm = mm;
            }
        }

        match page.validate() {
            Ok(()) => self.page = page,
            Err(_) => self.warn(
                "odt_invalid_page_layout",
                "Invalid ODT page layout was ignored during import",
            ),
        }
        Ok(())
    }

    fn read_track_changes_state(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        if attr_value_exact(start, b"word900:track-changes-recording")?.as_deref() == Some("true") {
            self.track_changes.recording = true;
        }
        Ok(())
    }

    fn start_paragraph(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        if self.active_text.is_some() {
            self.warn(
                "odt_nested_paragraph",
                "Nested paragraph content was ignored",
            );
            return Ok(());
        }
        if attr_value_exact(start, b"word900:block-type")?.as_deref() == Some("table-of-contents") {
            let title = attr_value_exact(start, b"word900:toc-title")?
                .map(|value| sanitize_toc_title(&value))
                .unwrap_or_else(|| "Contents".to_string());
            match attr_value_exact(start, b"word900:toc-entries")?
                .as_deref()
                .and_then(parse_toc_entries_metadata)
            {
                Some(entries) => {
                    self.active_text = Some(ActiveText::table_of_contents(title, entries));
                    return Ok(());
                }
                None => self.warn(
                    "odt_unsupported_toc_metadata",
                    "Unsupported 900Word table-of-contents metadata was imported as normal text",
                ),
            }
        }
        let style = attr_value(start, b"style-name")?.unwrap_or_else(|| "body".to_string());
        let (style, format) = paragraph_style_from_name(&style);
        let bookmark_id = self.read_bookmark_id(start)?;
        self.active_text = Some(ActiveText::paragraph(
            StyleId::from(style.as_str()),
            format,
            bookmark_id,
        ));
        Ok(())
    }

    fn start_heading(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        if self.active_text.is_some() {
            self.warn("odt_nested_heading", "Nested heading content was ignored");
            return Ok(());
        }
        let level = attr_value(start, b"outline-level")?
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(1)
            .clamp(1, 6);
        let bookmark_id = self.read_bookmark_id(start)?;
        self.active_text = Some(ActiveText::heading(level, bookmark_id));
        Ok(())
    }

    fn read_bookmark_id(&mut self, start: &BytesStart<'_>) -> Result<Option<String>, OdtError> {
        let name = match attr_value(start, b"bookmark-id")? {
            Some(value) => Some(value),
            None => attr_value(start, b"name")?,
        };
        let sanitized = name.as_deref().and_then(sanitize_bookmark_id);
        if name.is_some() && sanitized.is_none() {
            self.warn(
                "odt_unsafe_bookmark",
                "Unsafe bookmark name was stripped during import",
            );
        }
        Ok(sanitized)
    }

    fn read_inline_bookmark_id(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        let name = attr_value(start, b"name")?;
        let sanitized = name.as_deref().and_then(sanitize_bookmark_id);
        if name.is_some() && sanitized.is_none() {
            self.warn(
                "odt_unsafe_bookmark",
                "Unsafe bookmark name was stripped during import",
            );
            return Ok(());
        }
        if let (Some(active), Some(bookmark_id)) = (self.active_text.as_mut(), sanitized) {
            active.set_bookmark_id(bookmark_id);
        }
        Ok(())
    }

    fn start_span(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        let style_name = attr_value(start, b"style-name")?;
        let tracked_change = self.read_tracked_change(start)?;
        if let Some(active) = self.active_text.as_mut() {
            active.flush();
            let marks = style_name
                .as_deref()
                .map(marks_from_text_style)
                .unwrap_or_default();
            let inline_style = style_name
                .as_deref()
                .map(inline_style_from_text_style)
                .unwrap_or_default();
            active.mark_stack.push(marks);
            active.style_stack.push(inline_style);
            active.tracked_change_stack.push(tracked_change);
        }
        Ok(())
    }

    fn read_tracked_change(
        &mut self,
        start: &BytesStart<'_>,
    ) -> Result<Option<TrackedChange>, OdtError> {
        let id = attr_value_exact(start, b"word900:change-id")?;
        let kind = attr_value_exact(start, b"word900:change-kind")?;
        if id.is_none() && kind.is_none() {
            return Ok(None);
        }
        let Some(id) = id.and_then(|value| validate_tracked_change_id(&value).ok()) else {
            self.warn(
                "odt_unsafe_tracked_change",
                "Unsafe tracked-change metadata was ignored during import",
            );
            return Ok(None);
        };
        let Some(kind) = kind.as_deref().and_then(parse_tracked_change_kind) else {
            self.warn(
                "odt_unsupported_tracked_change",
                "Unsupported tracked-change kind was ignored during import",
            );
            return Ok(None);
        };
        let created_at = match attr_value_exact(start, b"word900:change-created-at")?
            .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
            .map(|value| value.with_timezone(&Utc))
        {
            Some(created_at) => created_at,
            None => {
                self.warn(
                    "odt_unsafe_tracked_change",
                    "Unsafe tracked-change metadata was ignored during import",
                );
                return Ok(None);
            }
        };
        let author =
            normalize_comment_author(attr_value_exact(start, b"word900:change-author")?.as_deref())
                .unwrap_or_else(|_| word_core::DEFAULT_TRACKED_CHANGE_AUTHOR.to_string());
        Ok(Some(TrackedChange {
            id,
            kind,
            author,
            created_at,
        }))
    }

    fn start_link(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        let href = attr_value(start, b"href")?;
        let sanitized = href.as_deref().and_then(sanitize_text_href);
        if href.is_some() && sanitized.is_none() {
            self.warn(
                "odt_unsafe_link",
                "Unsafe text link was stripped during import",
            );
        }
        if let Some(active) = self.active_text.as_mut() {
            active.flush();
            active.link_stack.push(sanitized);
        }
        Ok(())
    }

    fn start_annotation(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        let id = attr_value(start, b"comment-id")?.or(attr_value(start, b"name")?);
        let resolved = attr_value(start, b"resolved")?.as_deref() == Some("true");
        let Some(id) = id.and_then(|value| validate_comment_id(&value).ok()) else {
            self.warn(
                "odt_unsafe_comment",
                "Unsafe comment annotation was ignored during import",
            );
            self.unsupported_depth = 1;
            return Ok(());
        };
        self.active_annotation = Some(AnnotationParse::new(id, resolved));
        self.annotation_depth = 1;
        Ok(())
    }

    fn start_note(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        let odf_kind = attr_value(start, b"note-class")?.and_then(|value| parse_note_kind(&value));
        let word900_kind = attr_value_exact(start, b"word900:note-kind")?
            .and_then(|value| parse_note_kind(&value));
        let word900_id = attr_value_exact(start, b"word900:note-id")?
            .and_then(|value| validate_note_id(&value).ok());
        let kind = match (word900_kind, odf_kind) {
            (Some(word900_kind), Some(odf_kind)) if word900_kind == odf_kind => Some(word900_kind),
            _ => None,
        };
        self.active_note = Some(NoteParse::new(word900_id, kind));
        self.note_depth = 1;
        Ok(())
    }

    fn start_and_finish_empty_note(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        self.start_note(start)?;
        if self.active_note.is_some() {
            self.finish_note();
        }
        Ok(())
    }

    fn finish_note(&mut self) {
        let Some(note) = self.active_note.take() else {
            return;
        };
        self.note_depth = 0;
        let Some(kind) = note.kind else {
            self.warn(
                "odt_unsupported_note",
                "Unsupported note type was imported as visible text",
            );
            self.push_visible_note_fallback(&note);
            return;
        };
        let Some(id) = note
            .id
            .as_deref()
            .and_then(|value| validate_note_id(value).ok())
        else {
            self.warn(
                "odt_unsafe_note",
                "Unsafe note metadata was imported as visible text",
            );
            self.push_visible_note_fallback(&note);
            return;
        };
        let label = match validate_note_label(&note.citation) {
            Ok(label) => label,
            Err(_) => (self.notes.len() + 1).to_string(),
        };
        let body = match validate_note_body(&note.body) {
            Ok(body) => body,
            Err(_) => {
                self.warn(
                    "odt_invalid_note",
                    "Invalid note body was imported as visible text",
                );
                self.push_visible_note_fallback(&note);
                return;
            }
        };
        if note.overflowed {
            self.warn(
                "odt_invalid_note",
                "Invalid note body was imported as visible text",
            );
            self.push_visible_note_fallback(&note);
            return;
        }
        if self.notes.contains_key(&id) {
            self.warn(
                "odt_duplicate_note",
                "Duplicate note metadata was imported as visible text",
            );
            self.push_visible_note_fallback(&note);
            return;
        }
        if self.notes.len() >= word_core::MAX_NOTES {
            self.warn(
                "odt_too_many_notes",
                "Excess ODT note metadata was imported as visible text",
            );
            self.push_visible_note_fallback(&note);
            return;
        }
        if self.active_text.is_none() {
            self.warn(
                "odt_unanchored_note",
                "Note outside editable text was imported as visible text",
            );
            self.push_visible_note_fallback(&note);
            return;
        }
        self.notes.insert(
            id.clone(),
            Note {
                id: id.clone(),
                kind,
                body,
            },
        );
        if let Some(active) = self.active_text.as_mut() {
            active.push_note_reference(InlineNoteReference { id, kind, label });
        }
    }

    fn push_visible_note_fallback(&mut self, note: &NoteParse) {
        let mut text = String::new();
        let kind = note.kind.unwrap_or(NoteKind::Footnote);
        text.push('[');
        text.push_str(note_kind_name(kind));
        let citation = note.citation.trim();
        if !citation.is_empty() {
            text.push(' ');
            text.push_str(citation);
        }
        let body = note.body.trim();
        if !body.is_empty() {
            text.push_str(": ");
            text.push_str(body);
        }
        text.push(']');
        if let Some(active) = self.active_text.as_mut() {
            active.text.push_str(&text);
        } else {
            self.push_block(Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: StyleId::from("body"),
                format: Default::default(),
                inlines: vec![Inline::text(text)],
            }));
        }
    }

    fn start_and_finish_empty_annotation(
        &mut self,
        start: &BytesStart<'_>,
    ) -> Result<(), OdtError> {
        self.start_annotation(start)?;
        if self.active_annotation.is_some() {
            self.finish_annotation_anchor();
        }
        Ok(())
    }

    fn finish_annotation_anchor(&mut self) {
        let Some(annotation) = self.active_annotation.take() else {
            return;
        };
        self.annotation_depth = 0;
        let now = Utc::now();
        let author = normalize_comment_author(Some(&annotation.author)).unwrap_or_else(|_| {
            normalize_comment_author(None).expect("default comment author should be valid")
        });
        let body = match validate_comment_body(&annotation.body) {
            Ok(body) => body,
            Err(_) => {
                self.warn(
                    "odt_invalid_comment",
                    "Invalid comment body was ignored during import",
                );
                return;
            }
        };
        if self.active_text.is_none() {
            self.warn(
                "odt_unanchored_comment",
                "Comment annotation outside editable text was ignored during import",
            );
            return;
        }
        let created_at = annotation
            .created_at
            .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or(now);
        self.comments
            .entry(annotation.id.clone())
            .or_insert(CommentThread {
                id: annotation.id.clone(),
                author,
                body,
                created_at,
                updated_at: created_at,
                resolved: annotation.resolved,
            });
        if let Some(active) = self.active_text.as_mut() {
            active.push_comment_id(annotation.id);
        }
    }

    fn end_annotation_anchor(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        let Some(id) =
            attr_value(start, b"name")?.and_then(|value| validate_comment_id(&value).ok())
        else {
            return Ok(());
        };
        if let Some(active) = self.active_text.as_mut() {
            active.pop_comment_id(&id);
        }
        Ok(())
    }

    fn text(&mut self, value: &str) {
        if let Some(note) = self.active_note.as_mut() {
            note.text(value);
            return;
        }
        if let Some(annotation) = self.active_annotation.as_mut() {
            annotation.text(value);
            return;
        }
        if self.unsupported_depth == 0 {
            if let Some(active) = self.active_text.as_mut() {
                active.text.push_str(value);
            }
        }
    }

    fn start_page_field(&mut self, field: PageField) {
        self.push_page_field(field);
        self.unsupported_depth = 1;
    }

    fn push_page_field(&mut self, field: PageField) {
        if let Some(active) = self.active_text.as_mut() {
            active.push_field(field);
        }
    }

    fn start_list(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        self.list_counter += 1;
        let style = attr_value(start, b"style-name")?;
        let ordered = style
            .as_deref()
            .map(|value| {
                let lower = value.to_ascii_lowercase();
                lower == ORDERED_LIST_STYLE || lower.ends_with("-ordered")
            })
            .unwrap_or(false);
        let definition_id = style.unwrap_or_else(|| format!("list-{}", self.list_counter));
        self.lists.insert(
            definition_id.clone(),
            ListDefinition {
                ordered,
                marker: None,
            },
        );
        self.contexts.push(ParseContext::List {
            definition_id,
            ordered,
            items: Vec::new(),
        });
        Ok(())
    }

    fn start_frame(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        let name = attr_value(start, b"name")?;
        let alt_text = attr_value(start, b"title")?;
        let alignment = attr_value(start, b"alignment")?
            .as_deref()
            .and_then(parse_image_alignment)
            .unwrap_or_default();
        let scale_percent = attr_value(start, b"scale-percent")?
            .and_then(|value| value.parse::<u16>().ok())
            .map(|value| value.clamp(25, 200))
            .unwrap_or(100);
        let caption = attr_value(start, b"caption")?.filter(|value| !value.trim().is_empty());
        self.active_frame = Some(ImageFrame {
            _name: name,
            alt_text,
            href: None,
            presentation: ImagePresentation {
                alignment,
                scale_percent,
                caption,
            },
        });
        Ok(())
    }

    fn start_image(&mut self, start: &BytesStart<'_>) -> Result<(), OdtError> {
        let href = attr_value(start, b"href")?;
        if let Some(frame) = self.active_frame.as_mut() {
            frame.href = href;
        }
        Ok(())
    }

    fn finish_text_block(&mut self) {
        let Some(mut active) = self.active_text.take() else {
            return;
        };
        active.flush();

        let embedded = std::mem::take(&mut active.embedded_blocks);
        let is_image_paragraph = matches!(
            &active.kind,
            ActiveTextKind::Paragraph { style, .. } if style.as_str() == IMAGE_PARAGRAPH_STYLE
        );
        if is_image_paragraph {
            active
                .inlines
                .retain(|inline| !inline.text.trim().is_empty());
        }
        if active.inlines.is_empty() && embedded.len() == 1 && is_image_paragraph {
            self.push_block(embedded.into_iter().next().expect("checked length"));
            return;
        }
        if active.inlines.is_empty() && embedded.is_empty() && is_image_paragraph {
            return;
        }

        let block = match active.kind {
            ActiveTextKind::Paragraph {
                bookmark_id,
                style,
                format,
            } => Block::Paragraph(Paragraph {
                bookmark_id,
                style,
                format,
                inlines: active.inlines,
            }),
            ActiveTextKind::Heading { bookmark_id, level } => Block::Heading(Heading {
                bookmark_id,
                level,
                inlines: active.inlines,
            }),
            ActiveTextKind::TableOfContents { title, entries } => {
                if visible_table_of_contents_matches_metadata(&title, &entries, &active.inlines) {
                    Block::TableOfContents(TableOfContents { title, entries })
                } else {
                    self.warn(
                        "odt_unsupported_toc_metadata",
                        "Unsupported 900Word table-of-contents metadata was imported as normal text",
                    );
                    Block::Paragraph(Paragraph {
                        bookmark_id: None,
                        style: StyleId::from(TOC_PARAGRAPH_STYLE),
                        format: Default::default(),
                        inlines: active.inlines,
                    })
                }
            }
        };
        self.push_block(block);
        for block in embedded {
            self.push_block(block);
        }
    }

    fn finish_list_item(&mut self) {
        let Some(ParseContext::ListItem { level, blocks }) = self.contexts.pop() else {
            self.warn("odt_misnested_list_item", "Misnested list item was ignored");
            return;
        };
        match self.contexts.last_mut() {
            Some(ParseContext::List { items, .. }) => items.push(ListItem { level, blocks }),
            _ => self.warn(
                "odt_misnested_list_item",
                "List item outside a list was ignored",
            ),
        }
    }

    fn finish_list(&mut self) {
        let Some(ParseContext::List {
            definition_id,
            ordered,
            items,
        }) = self.contexts.pop()
        else {
            self.warn("odt_misnested_list", "Misnested list was ignored");
            return;
        };
        self.lists.insert(
            definition_id.clone(),
            ListDefinition {
                ordered,
                marker: None,
            },
        );
        self.push_block(Block::List(ListBlock {
            definition_id,
            items,
        }));
    }

    fn finish_table_cell(&mut self) {
        let Some(ParseContext::TableCell {
            presentation,
            blocks,
        }) = self.contexts.pop()
        else {
            self.warn(
                "odt_misnested_table_cell",
                "Misnested table cell was ignored",
            );
            return;
        };
        match self.contexts.last_mut() {
            Some(ParseContext::TableRow { cells }) => cells.push(TableCell {
                presentation,
                blocks,
            }),
            _ => self.warn(
                "odt_misnested_table_cell",
                "Table cell outside a table row was ignored",
            ),
        }
    }

    fn finish_table_row(&mut self) {
        let Some(ParseContext::TableRow { cells }) = self.contexts.pop() else {
            self.warn("odt_misnested_table_row", "Misnested table row was ignored");
            return;
        };
        match self.contexts.last_mut() {
            Some(ParseContext::Table { rows }) => rows.push(TableRow { cells }),
            _ => self.warn(
                "odt_misnested_table_row",
                "Table row outside a table was ignored",
            ),
        }
    }

    fn finish_table(&mut self) {
        let Some(ParseContext::Table { rows }) = self.contexts.pop() else {
            self.warn("odt_misnested_table", "Misnested table was ignored");
            return;
        };
        self.push_block(Block::Table(Table { rows }));
    }

    fn finish_frame(&mut self) {
        let Some(frame) = self.active_frame.take() else {
            return;
        };

        let Some(href) = frame.href else {
            self.warn(
                "odt_image_missing_href",
                "Image frame without a package href was ignored",
            );
            return;
        };

        if href.contains(':') || href.starts_with('/') || href.contains("..") {
            self.warn(
                "odt_unsafe_image_href",
                "Unsafe image reference was ignored",
            );
            return;
        }

        let Some(payload) = self.asset_payloads.get(&href) else {
            self.warn(
                "odt_missing_image_payload",
                "Image payload was missing from the package",
            );
            return;
        };

        let asset_id = payload.id.clone();
        self.assets.insert(
            asset_id.clone(),
            AssetRef {
                id: asset_id.clone(),
                media_type: payload.media_type.clone(),
                byte_len: payload.bytes.len(),
                bytes: payload.bytes.clone(),
                original_name: None,
            },
        );

        let image = Block::Image(ImageBlock {
            asset_id,
            presentation: frame.presentation,
            alt_text: frame.alt_text,
        });

        if let Some(active) = self.active_text.as_mut() {
            active.embedded_blocks.push(image);
        } else {
            self.push_block(image);
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.contexts.last_mut() {
            Some(ParseContext::ListItem { blocks, .. })
            | Some(ParseContext::TableCell { blocks, .. }) => blocks.push(block),
            Some(_) => self.warn(
                "odt_unsupported_structure",
                "Block appeared in an unsupported ODT container and was ignored",
            ),
            None => self.blocks.push(block),
        }
    }

    fn warn(&mut self, code: &str, message: &str) {
        self.warnings.push(DocumentWarning {
            code: code.to_string(),
            message: message.to_string(),
        });
    }

    fn warn_unsupported_element(&mut self, name: &[u8]) {
        let local = String::from_utf8_lossy(name).into_owned();
        if self.unsupported_elements.insert(local.clone()) {
            self.warnings.push(DocumentWarning {
                code: "odt_unsupported_element".to_string(),
                message: format!("Unsupported ODT element '{local}' was ignored during import"),
            });
        }
    }

    fn into_document(self) -> Document {
        let mut document = Document::new_untitled();
        document.sections = vec![Section {
            blocks: self.blocks,
            page: self.page,
            ..Section::default()
        }];
        document.styles = self.styles;
        document.assets = self.assets;
        document.comments = self.comments;
        document.notes = self.notes;
        document.lists = self.lists;
        document.track_changes = self.track_changes;
        document.warnings = self.warnings;
        prune_unanchored_imported_comments(&mut document);
        document
    }
}

#[derive(Debug)]
enum ParseContext {
    List {
        definition_id: String,
        ordered: bool,
        items: Vec<ListItem>,
    },
    ListItem {
        level: u8,
        blocks: Vec<Block>,
    },
    Table {
        rows: Vec<TableRow>,
    },
    TableRow {
        cells: Vec<TableCell>,
    },
    TableCell {
        presentation: TableCellPresentation,
        blocks: Vec<Block>,
    },
}

#[derive(Debug)]
struct ImageFrame {
    _name: Option<String>,
    alt_text: Option<String>,
    href: Option<String>,
    presentation: ImagePresentation,
}

#[derive(Debug)]
struct AnnotationParse {
    id: String,
    author: String,
    body: String,
    created_at: Option<String>,
    resolved: bool,
    field: AnnotationField,
}

impl AnnotationParse {
    fn new(id: String, resolved: bool) -> Self {
        Self {
            id,
            author: String::new(),
            body: String::new(),
            created_at: None,
            resolved,
            field: AnnotationField::None,
        }
    }

    fn start(&mut self, name: &[u8]) {
        self.field = match name {
            b"creator" => AnnotationField::Author,
            b"date" => AnnotationField::Date,
            b"p" => AnnotationField::Body,
            _ => AnnotationField::None,
        };
    }

    fn end(&mut self, name: &[u8]) {
        if name == b"creator" || name == b"date" || name == b"p" {
            self.field = AnnotationField::None;
        }
    }

    fn text(&mut self, value: &str) {
        match self.field {
            AnnotationField::Author => self.author.push_str(value),
            AnnotationField::Date => {
                let current = self.created_at.get_or_insert_with(String::new);
                current.push_str(value);
            }
            AnnotationField::Body => self.body.push_str(value),
            AnnotationField::None => {}
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum AnnotationField {
    None,
    Author,
    Date,
    Body,
}

#[derive(Debug)]
struct NoteParse {
    id: Option<String>,
    kind: Option<NoteKind>,
    citation: String,
    body: String,
    field: NoteField,
    overflowed: bool,
}

impl NoteParse {
    fn new(id: Option<String>, kind: Option<NoteKind>) -> Self {
        Self {
            id,
            kind,
            citation: String::new(),
            body: String::new(),
            field: NoteField::None,
            overflowed: false,
        }
    }

    fn start(&mut self, name: &[u8]) {
        self.field = match name {
            b"note-citation" => NoteField::Citation,
            b"p" if self.field == NoteField::Body => NoteField::Body,
            b"note-body" => NoteField::Body,
            _ => self.field,
        };
    }

    fn empty(&mut self, name: &[u8]) {
        match name {
            b"s" => self.text(" "),
            b"line-break" => self.text("\n"),
            _ => {}
        }
    }

    fn end(&mut self, name: &[u8]) {
        match name {
            b"note-citation" | b"note-body" => self.field = NoteField::None,
            _ => {}
        }
    }

    fn text(&mut self, value: &str) {
        match self.field {
            NoteField::Citation => {
                if push_bounded_text(&mut self.citation, value, word_core::MAX_NOTE_LABEL_CHARS) {
                    self.overflowed = true;
                }
            }
            NoteField::Body => {
                if push_bounded_text(&mut self.body, value, word_core::MAX_NOTE_BODY_CHARS) {
                    self.overflowed = true;
                }
            }
            NoteField::None => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoteField {
    None,
    Citation,
    Body,
}

fn push_bounded_text(output: &mut String, value: &str, max_chars: usize) -> bool {
    let remaining = max_chars.saturating_sub(output.chars().count());
    if remaining == 0 {
        return !value.is_empty();
    }
    let mut overflowed = false;
    for (index, ch) in value.chars().enumerate() {
        if index >= remaining {
            overflowed = true;
            break;
        }
        output.push(ch);
    }
    overflowed
}

#[derive(Debug)]
enum ActiveTextKind {
    Paragraph {
        bookmark_id: Option<String>,
        style: StyleId,
        format: ParagraphFormat,
    },
    Heading {
        bookmark_id: Option<String>,
        level: u8,
    },
    TableOfContents {
        title: String,
        entries: Vec<TableOfContentsEntry>,
    },
}

#[derive(Debug)]
struct ActiveText {
    kind: ActiveTextKind,
    text: String,
    inlines: Vec<Inline>,
    mark_stack: Vec<Vec<InlineMark>>,
    style_stack: Vec<InlineStyle>,
    link_stack: Vec<Option<String>>,
    comment_stack: Vec<String>,
    tracked_change_stack: Vec<Option<TrackedChange>>,
    embedded_blocks: Vec<Block>,
}

impl ActiveText {
    fn paragraph(style: StyleId, format: ParagraphFormat, bookmark_id: Option<String>) -> Self {
        Self {
            kind: ActiveTextKind::Paragraph {
                bookmark_id,
                style,
                format,
            },
            text: String::new(),
            inlines: Vec::new(),
            mark_stack: Vec::new(),
            style_stack: Vec::new(),
            link_stack: Vec::new(),
            comment_stack: Vec::new(),
            tracked_change_stack: Vec::new(),
            embedded_blocks: Vec::new(),
        }
    }

    fn heading(level: u8, bookmark_id: Option<String>) -> Self {
        Self {
            kind: ActiveTextKind::Heading { bookmark_id, level },
            text: String::new(),
            inlines: Vec::new(),
            mark_stack: Vec::new(),
            style_stack: Vec::new(),
            link_stack: Vec::new(),
            comment_stack: Vec::new(),
            tracked_change_stack: Vec::new(),
            embedded_blocks: Vec::new(),
        }
    }

    fn table_of_contents(title: String, entries: Vec<TableOfContentsEntry>) -> Self {
        Self {
            kind: ActiveTextKind::TableOfContents { title, entries },
            text: String::new(),
            inlines: Vec::new(),
            mark_stack: Vec::new(),
            style_stack: Vec::new(),
            link_stack: Vec::new(),
            comment_stack: Vec::new(),
            tracked_change_stack: Vec::new(),
            embedded_blocks: Vec::new(),
        }
    }

    fn set_bookmark_id(&mut self, bookmark_id: String) {
        match &mut self.kind {
            ActiveTextKind::Paragraph {
                bookmark_id: slot, ..
            }
            | ActiveTextKind::Heading {
                bookmark_id: slot, ..
            } => {
                if slot.is_none() {
                    *slot = Some(bookmark_id);
                }
            }
            ActiveTextKind::TableOfContents { .. } => {}
        }
    }

    fn flush(&mut self) {
        if self.text.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.text);
        self.inlines.push(Inline {
            text,
            marks: self.active_marks(),
            link: self.active_link(),
            comment_ids: self.active_comment_ids(),
            style: self.active_style(),
            field: None,
            note_reference: None,
            tracked_change: self.active_tracked_change(),
        });
    }

    fn push_field(&mut self, field: PageField) {
        self.flush();
        self.inlines.push(Inline {
            text: field.fallback_text().to_string(),
            marks: self.active_marks(),
            link: self.active_link(),
            comment_ids: self.active_comment_ids(),
            style: self.active_style(),
            field: Some(field),
            note_reference: None,
            tracked_change: self.active_tracked_change(),
        });
    }

    fn push_note_reference(&mut self, reference: InlineNoteReference) {
        self.flush();
        self.inlines.push(Inline {
            text: reference.label.clone(),
            marks: self.active_marks(),
            link: self.active_link(),
            comment_ids: self.active_comment_ids(),
            style: self.active_style(),
            field: None,
            note_reference: Some(reference),
            tracked_change: self.active_tracked_change(),
        });
    }

    fn push_comment_id(&mut self, comment_id: String) {
        self.flush();
        if !self.comment_stack.contains(&comment_id) {
            self.comment_stack.push(comment_id);
        }
    }

    fn pop_comment_id(&mut self, comment_id: &str) {
        self.flush();
        if let Some(index) = self
            .comment_stack
            .iter()
            .rposition(|active| active == comment_id)
        {
            self.comment_stack.remove(index);
        }
    }

    fn active_marks(&self) -> Vec<InlineMark> {
        let mut active = BTreeSet::new();
        for marks in &self.mark_stack {
            for mark in marks {
                active.insert(mark_order(*mark));
            }
        }
        active.into_iter().map(mark_from_order).collect()
    }

    fn active_link(&self) -> Option<String> {
        self.link_stack.iter().rev().find_map(|href| href.clone())
    }

    fn active_comment_ids(&self) -> Vec<String> {
        self.comment_stack.clone()
    }

    fn active_tracked_change(&self) -> Option<TrackedChange> {
        self.tracked_change_stack
            .iter()
            .rev()
            .find_map(|change| change.clone())
    }

    fn active_style(&self) -> InlineStyle {
        let mut style = InlineStyle::default();
        for candidate in &self.style_stack {
            if candidate.font_family.is_some() {
                style.font_family = candidate.font_family.clone();
            }
            if candidate.font_size_pt.is_some() {
                style.font_size_pt = candidate.font_size_pt;
            }
            if candidate.text_color.is_some() {
                style.text_color = candidate.text_color.clone();
            }
            if candidate.highlight_color.is_some() {
                style.highlight_color = candidate.highlight_color.clone();
            }
        }
        style
    }
}

fn extract_meta_title(meta: &str) -> Result<Option<String>, OdtError> {
    let mut reader = Reader::from_str(meta);
    reader.config_mut().trim_text(false);
    let mut in_title = false;
    let mut title = String::new();

    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error("meta.xml", err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"title" => {
                in_title = true;
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"title" => break,
            Event::Text(text) if in_title => {
                title.push_str(
                    &text
                        .xml10_content()
                        .map_err(|err| xml_error("meta.xml", err))?,
                );
            }
            Event::Eof => break,
            _ => {}
        }
    }

    if title.is_empty() {
        Ok(None)
    } else {
        Ok(Some(title))
    }
}

fn attr_value(start: &BytesStart<'_>, local: &[u8]) -> Result<Option<String>, OdtError> {
    for attr in start.attributes().with_checks(true) {
        let attr = attr.map_err(|err| xml_error("content.xml", err))?;
        if local_name(attr.key.as_ref()) == local {
            return Ok(Some(
                attr.decode_and_unescape_value(start.decoder())
                    .map_err(|err| xml_error("content.xml", err))?
                    .into_owned(),
            ));
        }
    }
    Ok(None)
}

fn attr_value_exact(start: &BytesStart<'_>, name: &[u8]) -> Result<Option<String>, OdtError> {
    for attr in start.attributes().with_checks(true) {
        let attr = attr.map_err(|err| xml_error("content.xml", err))?;
        if attr.key.as_ref() == name {
            return Ok(Some(
                attr.decode_and_unescape_value(start.decoder())
                    .map_err(|err| xml_error("content.xml", err))?
                    .into_owned(),
            ));
        }
    }
    Ok(None)
}

fn parse_table_cell_presentation(
    start: &BytesStart<'_>,
) -> Result<TableCellPresentation, OdtError> {
    let background_color = attr_value_exact(start, b"word900:cell-background-color")?
        .as_deref()
        .and_then(sanitize_table_cell_background_color);
    let text_alignment = attr_value_exact(start, b"word900:cell-text-align")?
        .as_deref()
        .and_then(parse_table_cell_alignment);
    let border = attr_value_exact(start, b"word900:cell-border")?
        .as_deref()
        .and_then(parse_table_cell_border)
        .unwrap_or_default();
    Ok(TableCellPresentation {
        background_color,
        text_alignment,
        border,
    })
}

fn parse_table_cell_alignment(value: &str) -> Option<ParagraphAlignment> {
    match value {
        "left" => Some(ParagraphAlignment::Left),
        "center" => Some(ParagraphAlignment::Center),
        "right" => Some(ParagraphAlignment::Right),
        "justify" => Some(ParagraphAlignment::Justify),
        _ => None,
    }
}

fn parse_table_cell_border(value: &str) -> Option<TableCellBorder> {
    match value {
        "visible" => Some(TableCellBorder::Visible),
        "hidden" => Some(TableCellBorder::Hidden),
        _ => None,
    }
}

fn parse_mm_attr(value: &str) -> Option<u16> {
    let trimmed = value.trim();
    let number = trimmed.strip_suffix("mm")?.trim();
    let parsed = number.parse::<u16>().ok()?;
    Some(parsed)
}

fn parse_signed_mm_attr(value: &str) -> Option<i16> {
    let trimmed = value.trim();
    let number = trimmed.strip_suffix("mm")?.trim();
    number.parse::<i16>().ok()
}

fn parse_line_height_per_mille(value: &str) -> Option<u16> {
    let trimmed = value.trim();
    let percent = trimmed.strip_suffix('%')?.trim();
    let parsed = percent.parse::<u16>().ok()?;
    Some(parsed.saturating_mul(10))
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn paragraph_style_name(base_style: &str, format: &ParagraphFormat) -> String {
    let mut tokens = vec![format!("base-{}", encode_style_token(base_style))];
    if let Some(alignment) = format.alignment {
        tokens.push(format!(
            "a-{}",
            match alignment {
                ParagraphAlignment::Left => "left",
                ParagraphAlignment::Center => "center",
                ParagraphAlignment::Right => "right",
                ParagraphAlignment::Justify => "justify",
            }
        ));
    }
    if let Some(value) = format.line_spacing_per_mille {
        tokens.push(format!("ls-{value}"));
    }
    if let Some(value) = format.spacing_before_mm {
        tokens.push(format!("sb-{value}"));
    }
    if let Some(value) = format.spacing_after_mm {
        tokens.push(format!("sa-{value}"));
    }
    if let Some(value) = format.indent_start_mm {
        tokens.push(format!("is-{value}"));
    }
    if let Some(value) = format.indent_end_mm {
        tokens.push(format!("ie-{value}"));
    }
    if let Some(value) = format.first_line_indent_mm {
        tokens.push(format!("fi-{}", encode_signed_number(value)));
    }
    format!("{PARAGRAPH_STYLE_PREFIX}-{}", tokens.join("-"))
}

fn paragraph_style_from_name(style_name: &str) -> (String, ParagraphFormat) {
    let Some(tokens) = style_name.strip_prefix(&format!("{PARAGRAPH_STYLE_PREFIX}-")) else {
        return (style_name.to_string(), ParagraphFormat::default());
    };

    let mut base_style = "body".to_string();
    let mut format = ParagraphFormat::default();
    let parts: Vec<&str> = tokens.split('-').collect();
    let mut index = 0;
    while index + 1 < parts.len() {
        match parts[index] {
            "base" => base_style = decode_style_token(parts[index + 1]),
            "a" => {
                format.alignment = match parts[index + 1] {
                    "left" => Some(ParagraphAlignment::Left),
                    "center" => Some(ParagraphAlignment::Center),
                    "right" => Some(ParagraphAlignment::Right),
                    "justify" => Some(ParagraphAlignment::Justify),
                    _ => None,
                };
            }
            "ls" => format.line_spacing_per_mille = parts[index + 1].parse().ok(),
            "sb" => format.spacing_before_mm = parts[index + 1].parse().ok(),
            "sa" => format.spacing_after_mm = parts[index + 1].parse().ok(),
            "is" => format.indent_start_mm = parts[index + 1].parse().ok(),
            "ie" => format.indent_end_mm = parts[index + 1].parse().ok(),
            "fi" => format.first_line_indent_mm = decode_signed_number(parts[index + 1]),
            _ => {}
        }
        index += 2;
    }
    (base_style, format)
}

fn text_style_name(marks: &[InlineMark], inline_style: &InlineStyle) -> Option<String> {
    if marks.is_empty() && inline_style.is_default() {
        return None;
    }
    let orders: BTreeSet<u8> = marks.iter().copied().map(mark_order).collect();
    let mut tokens = Vec::new();
    for order in orders.iter() {
        tokens.push(
            match mark_from_order(*order) {
                InlineMark::Bold => "b",
                InlineMark::Italic => "i",
                InlineMark::Underline => "u",
                InlineMark::Strikethrough => "strike",
                InlineMark::Superscript => "super",
                InlineMark::Subscript => "sub",
            }
            .to_string(),
        );
    }
    if let Some(value) = inline_style.font_family.as_deref() {
        tokens.push(format!("ff-{}", encode_style_token(value)));
    }
    if let Some(value) = inline_style.font_size_pt {
        tokens.push(format!("fs-{value}"));
    }
    if let Some(value) = inline_style.text_color.as_deref().and_then(color_token) {
        tokens.push(format!("tc-{value}"));
    }
    if let Some(value) = inline_style
        .highlight_color
        .as_deref()
        .and_then(color_token)
    {
        tokens.push(format!("hc-{value}"));
    }
    Some(format!("{TEXT_STYLE_PREFIX}-{}", tokens.join("-")))
}

fn marks_from_text_style(style_name: &str) -> Vec<InlineMark> {
    let Some(tokens) = style_name.strip_prefix(&format!("{TEXT_STYLE_PREFIX}-")) else {
        return Vec::new();
    };

    let mut marks = BTreeSet::new();
    for token in tokens.split('-') {
        let mark = match token {
            "b" => Some(InlineMark::Bold),
            "i" => Some(InlineMark::Italic),
            "u" => Some(InlineMark::Underline),
            "strike" => Some(InlineMark::Strikethrough),
            "super" => Some(InlineMark::Superscript),
            "sub" => Some(InlineMark::Subscript),
            _ => None,
        };
        if let Some(mark) = mark {
            marks.insert(mark_order(mark));
        }
    }
    marks.into_iter().map(mark_from_order).collect()
}

fn is_supported_generated_text_style(style_name: &str) -> bool {
    let Some(tokens) = style_name.strip_prefix(&format!("{TEXT_STYLE_PREFIX}-")) else {
        return false;
    };
    if tokens.is_empty() {
        return false;
    }

    let parts: Vec<&str> = tokens.split('-').collect();
    let mut index = 0;
    while index < parts.len() {
        match parts[index] {
            "b" | "i" | "u" | "strike" | "super" | "sub" => index += 1,
            "ff" => {
                if index + 1 >= parts.len() || parts[index + 1].is_empty() {
                    return false;
                }
                index += 2;
            }
            "fs" => {
                if index + 1 >= parts.len() || parts[index + 1].parse::<u16>().is_err() {
                    return false;
                }
                index += 2;
            }
            "tc" | "hc" => {
                if index + 1 >= parts.len() {
                    return false;
                }
                let color = parts[index + 1];
                if color.len() != 6 || !color.chars().all(|ch| ch.is_ascii_hexdigit()) {
                    return false;
                }
                index += 2;
            }
            _ => return false,
        }
    }
    true
}

fn inline_style_from_text_style(style_name: &str) -> InlineStyle {
    let Some(tokens) = style_name.strip_prefix(&format!("{TEXT_STYLE_PREFIX}-")) else {
        return InlineStyle::default();
    };

    let mut style = InlineStyle::default();
    let parts: Vec<&str> = tokens.split('-').collect();
    let mut index = 0;
    while index + 1 < parts.len() {
        match parts[index] {
            "ff" => style.font_family = Some(decode_style_token(parts[index + 1])),
            "fs" => style.font_size_pt = parts[index + 1].parse().ok(),
            "tc" => style.text_color = Some(format!("#{}", parts[index + 1])),
            "hc" => style.highlight_color = Some(format!("#{}", parts[index + 1])),
            _ => {}
        }
        index += 1;
    }
    style
}

fn encode_style_token(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn decode_style_token(value: &str) -> String {
    value.replace('_', "-")
}

fn color_token(value: &str) -> Option<String> {
    let stripped = value.strip_prefix('#')?;
    if stripped.len() == 6 && stripped.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Some(stripped.to_ascii_lowercase())
    } else {
        None
    }
}

fn encode_signed_number(value: i16) -> String {
    if value < 0 {
        format!("n{}", value.abs())
    } else {
        value.to_string()
    }
}

fn decode_signed_number(value: &str) -> Option<i16> {
    if let Some(rest) = value.strip_prefix('n') {
        rest.parse::<i16>().ok().map(|number| -number)
    } else {
        value.parse::<i16>().ok()
    }
}

fn mark_order(mark: InlineMark) -> u8 {
    match mark {
        InlineMark::Bold => 0,
        InlineMark::Italic => 1,
        InlineMark::Underline => 2,
        InlineMark::Strikethrough => 3,
        InlineMark::Superscript => 4,
        InlineMark::Subscript => 5,
    }
}

fn mark_from_order(order: u8) -> InlineMark {
    match order {
        0 => InlineMark::Bold,
        1 => InlineMark::Italic,
        2 => InlineMark::Underline,
        3 => InlineMark::Strikethrough,
        4 => InlineMark::Superscript,
        _ => InlineMark::Subscript,
    }
}

fn sanitize_text_href(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(fragment) = trimmed.strip_prefix('#') {
        return sanitize_bookmark_id(fragment).map(|id| format!("#{id}"));
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:")
    {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn sanitize_toc_title(value: &str) -> String {
    let title = value
        .chars()
        .filter(|ch| !ch.is_control())
        .take(word_core::MAX_TABLE_OF_CONTENTS_TITLE_CHARS)
        .collect::<String>();
    let trimmed = title.trim();
    if trimmed.is_empty() {
        "Contents".to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_toc_entries_metadata(value: &str) -> Option<Vec<TableOfContentsEntry>> {
    if value.len() > 16 * 1024 {
        return None;
    }
    let entries = serde_json::from_str::<Vec<TableOfContentsEntry>>(value).ok()?;
    if entries.len() > word_core::MAX_TABLE_OF_CONTENTS_ENTRIES {
        return None;
    }
    let mut safe_entries = Vec::with_capacity(entries.len());
    for entry in entries {
        let text = sanitize_toc_entry_text(&entry.text)?;
        if !(1..=3).contains(&entry.level) {
            return None;
        }
        let target_bookmark_id = sanitize_bookmark_id(&entry.target_bookmark_id)?;
        safe_entries.push(TableOfContentsEntry {
            level: entry.level,
            text,
            target_bookmark_id,
        });
    }
    Some(safe_entries)
}

fn sanitize_toc_entry_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed != value {
        return None;
    }
    if trimmed.chars().count() > word_core::MAX_TABLE_OF_CONTENTS_ENTRY_TEXT_CHARS {
        return None;
    }
    if trimmed.chars().any(|ch| ch.is_control()) {
        return None;
    }
    Some(trimmed.to_string())
}

fn visible_table_of_contents_matches_metadata(
    title: &str,
    entries: &[TableOfContentsEntry],
    inlines: &[Inline],
) -> bool {
    let visible = inlines
        .iter()
        .flat_map(|inline| {
            let link = inline.link.clone();
            inline.text.chars().map(move |ch| (ch, link.clone()))
        })
        .collect::<Vec<_>>();
    let visible_text = visible.iter().map(|(ch, _)| *ch).collect::<String>();
    let mut expected = title.trim().to_string();
    let mut linked_ranges = Vec::new();

    for entry in entries {
        expected.push('\n');
        for _ in 1..entry.level.clamp(1, 3) {
            expected.push_str("  ");
        }
        let start = expected.chars().count();
        expected.push_str(&entry.text);
        let end = expected.chars().count();
        linked_ranges.push((start, end, format!("#{}", entry.target_bookmark_id)));
    }

    if visible_text != expected {
        return false;
    }

    for (index, (_, link)) in visible.iter().enumerate() {
        let expected_link = linked_ranges
            .iter()
            .find(|(start, end, _)| *start <= index && index < *end)
            .map(|(_, _, target)| target.as_str());
        if link.as_deref() != expected_link {
            return false;
        }
    }

    true
}

fn sanitize_bookmark_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let mut chars = trimmed.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    if trimmed.len() > 64 {
        return None;
    }
    if chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_') {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn parse_image_alignment(value: &str) -> Option<ImageAlignment> {
    match value {
        "inline" => Some(ImageAlignment::Inline),
        "left" => Some(ImageAlignment::Left),
        "center" => Some(ImageAlignment::Center),
        "right" => Some(ImageAlignment::Right),
        _ => None,
    }
}

fn detect_image_media_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]) {
        return Some("image/png");
    }
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
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

fn generic_imported_image_id(index: usize, media_type: &str) -> String {
    let extension = match media_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "bin",
    };
    format!("image-{index}.{extension}")
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn xml_error(name: &str, err: impl Display) -> OdtError {
    OdtError::Xml {
        name: name.to_string(),
        message: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use word_core::{InlineNoteReference, NoteKind, Style, StyleKind};

    const SAMPLE_PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1,
        13, 10, 45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    #[test]
    fn generated_odt_round_trips_mvp_blocks_and_multilingual_text() {
        let document = sample_document();

        let bytes = write_odt_bytes(&document).expect("write should succeed");
        let parsed = read_odt_bytes(&bytes).expect("read should succeed");

        assert_eq!(parsed.meta.title, "ODT MVP Sample");
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        assert_eq!(parsed.sections[0].blocks.len(), 5);
        assert_eq!(
            parsed
                .style(&StyleId::from("caption"))
                .map(|style| style.name.as_str()),
            Some("Caption")
        );

        let Block::Heading(heading) = &parsed.sections[0].blocks[0] else {
            panic!("first block should be a heading");
        };
        assert_eq!(heading.level, 1);

        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[1] else {
            panic!("second block should be a paragraph");
        };
        assert_eq!(paragraph.style.as_str(), "caption");
        assert_eq!(paragraph.inlines[0].marks, vec![InlineMark::Bold]);
        assert_eq!(
            paragraph.inlines[1].marks,
            vec![InlineMark::Italic, InlineMark::Underline]
        );
        assert_eq!(
            paragraph.inlines[1].link.as_deref(),
            Some("https://example.invalid/reference")
        );
        assert!(paragraph
            .inlines
            .iter()
            .any(|inline| inline.text.contains("العربية")));
        assert!(paragraph
            .inlines
            .iter()
            .any(|inline| inline.text.contains("中文")));

        let Block::List(list) = &parsed.sections[0].blocks[2] else {
            panic!("third block should be a list");
        };
        assert_eq!(list.items.len(), 2);
        assert_eq!(
            parsed
                .lists
                .get(&list.definition_id)
                .map(|definition| definition.ordered),
            Some(false)
        );

        let Block::Table(table) = &parsed.sections[0].blocks[3] else {
            panic!("fourth block should be a table");
        };
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells.len(), 2);

        let Block::Image(image) = &parsed.sections[0].blocks[4] else {
            panic!("fifth block should be an image");
        };
        assert_eq!(image.alt_text.as_deref(), Some("Synthetic sample image"));
        assert_eq!(image.presentation.alignment, ImageAlignment::Center);
        assert_eq!(image.presentation.scale_percent, 75);
        assert_eq!(
            image.presentation.caption.as_deref(),
            Some("Synthetic caption")
        );
        let asset = parsed
            .assets
            .get(&image.asset_id)
            .expect("image asset should be present");
        assert_eq!(image.asset_id, "image-1.png");
        assert_eq!(asset.media_type, "image/png");
        assert_eq!(asset.bytes, SAMPLE_PNG);
        assert_eq!(asset.original_name, None);
        assert!(!parsed.assets.contains_key("sample.png"));

        let reparsed =
            read_odt_bytes(&write_odt_bytes(&parsed).expect("rewrite should succeed")).unwrap();
        assert_eq!(
            reparsed.sections[0].blocks.len(),
            parsed.sections[0].blocks.len()
        );
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

    #[test]
    fn xml_entity_declaration_is_rejected() {
        let bytes = test_package_with_content(
            r#"<?xml version="1.0"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///etc/passwd">]><office:document-content/>"#,
        );

        let err = validate_odt_package(&bytes, PackageLimits::default())
            .expect_err("entity declaration should fail");

        assert!(matches!(err, OdtError::XmlEntityDeclaration { .. }));
    }

    #[test]
    fn oversized_image_entry_is_rejected() {
        let bytes = test_package_with_image(vec![1, 2, 3, 4, 5]);
        let limits = PackageLimits {
            max_image_size: 4,
            ..PackageLimits::default()
        };

        let err = validate_odt_package(&bytes, limits).expect_err("oversized image should fail");

        assert!(matches!(err, OdtError::ImageTooLarge { .. }));
    }

    #[test]
    fn unsupported_image_payload_type_is_rejected() {
        let bytes = test_package_with_image(b"<svg><script/></svg>".to_vec());

        let err = validate_odt_package(&bytes, PackageLimits::default())
            .expect_err("svg payload should fail");

        assert!(matches!(err, OdtError::UnsupportedImageType { .. }));
    }

    #[test]
    fn missing_mimetype_is_rejected() {
        let bytes = test_package_without_mimetype(
            r#"<?xml version="1.0"?><office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"/>"#,
        );

        let err = validate_odt_package(&bytes, PackageLimits::default())
            .expect_err("missing mimetype should fail");

        assert!(matches!(err, OdtError::InvalidMimeType));
    }

    #[test]
    fn package_size_limit_is_enforced() {
        let bytes = test_package_with_content(
            r#"<?xml version="1.0"?><office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"/>"#,
        );
        let limits = PackageLimits {
            max_package_size: bytes.len() as u64 - 1,
            ..PackageLimits::default()
        };

        let err = validate_odt_package(&bytes, limits).expect_err("oversized package should fail");

        assert!(matches!(err, OdtError::PackageTooLarge));
    }

    #[test]
    fn path_depth_limit_is_enforced() {
        let bytes = test_package_with_content(
            r#"<?xml version="1.0"?><office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"/>"#,
        );
        let limits = PackageLimits {
            max_path_depth: 0,
            ..PackageLimits::default()
        };

        let err = validate_odt_package(&bytes, limits).expect_err("deep path should fail");

        assert!(matches!(err, OdtError::PathTooDeep { .. }));
    }

    #[test]
    fn unsupported_odt_element_imports_with_warning() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              office:version="1.3">
              <office:body><office:text>
                <text:p>Visible text</text:p>
                <text:unknown-element>Unsupported payload</text:unknown-element>
              </office:text></office:body>
            </office:document-content>"#;

        let parsed = read_odt_bytes(&test_package_with_content(content)).expect("package parses");

        assert_eq!(parsed.sections[0].blocks.len(), 1);
        assert!(!parsed.sections[0].blocks.iter().any(
            |block| matches!(block, Block::Paragraph(paragraph) if paragraph
                .inlines
                .iter()
                .any(|inline| inline.text.contains("Unsupported payload")))
        ));
        assert!(parsed
            .warnings
            .iter()
            .any(|warning| warning.code == "odt_unsupported_element"));
    }

    #[test]
    fn page_break_round_trips_as_block() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: StyleId::from("body"),
                format: Default::default(),
                inlines: vec![Inline::text("Before")],
            }),
            Block::PageBreak,
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: StyleId::from("body"),
                format: Default::default(),
                inlines: vec![Inline::text("After")],
            }),
        ];

        let parsed =
            read_odt_bytes(&write_odt_bytes(&document).expect("write should succeed")).unwrap();

        assert!(matches!(parsed.sections[0].blocks[1], Block::PageBreak));
    }

    #[test]
    fn page_setup_round_trips_as_content_layout_metadata() {
        let mut document = Document::new_untitled();
        document.sections[0].page = PageSetup {
            width_mm: 148,
            height_mm: 210,
            margin_top_mm: 18,
            margin_right_mm: 16,
            margin_bottom_mm: 18,
            margin_left_mm: 16,
        };

        let bytes = write_odt_bytes(&document).expect("write should succeed");
        let parsed = read_odt_bytes(&bytes).expect("read should succeed");

        assert_eq!(parsed.sections[0].page, document.sections[0].page);
    }

    #[test]
    fn page_regions_and_fields_round_trip_through_styles_xml() {
        let mut document = Document::new_untitled();
        document.sections[0].page_regions.different_first_page = true;
        document.sections[0].page_regions.header.blocks =
            vec![PageRegionBlock::Paragraph(PageRegionParagraph {
                inlines: vec![
                    Inline::text("Page "),
                    Inline::field(PageField::PageNumber),
                    Inline::text(" of "),
                    Inline::field(PageField::PageCount),
                ],
            })];
        document.sections[0].page_regions.footer.blocks =
            vec![PageRegionBlock::Paragraph(PageRegionParagraph {
                inlines: vec![Inline::text("Updated "), Inline::field(PageField::Date)],
            })];

        let parsed =
            read_odt_bytes(&write_odt_bytes(&document).expect("write should succeed")).unwrap();

        assert!(parsed.sections[0].page_regions.different_first_page);
        let PageRegionBlock::Paragraph(header) = &parsed.sections[0].page_regions.header.blocks[0];
        assert_eq!(header.inlines[1].field, Some(PageField::PageNumber));
        assert_eq!(header.inlines[3].field, Some(PageField::PageCount));
        let PageRegionBlock::Paragraph(footer) = &parsed.sections[0].page_regions.footer.blocks[0];
        assert_eq!(footer.inlines[1].field, Some(PageField::Date));
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    }

    #[test]
    fn unsupported_external_header_imports_read_only_and_refuses_rewrite() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              office:version="1.3">
              <office:body><office:text><text:p>Body</text:p></office:text></office:body>
            </office:document-content>"#;
        let styles = r#"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-styles
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
              xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
              office:version="1.3">
              <office:master-styles>
                <style:master-page>
                  <style:header>
                    <text:p>Visible header</text:p>
                    <table:table><table:table-row><table:table-cell><text:p>Complex</text:p></table:table-cell></table:table-row></table:table>
                  </style:header>
                </style:master-page>
              </office:master-styles>
            </office:document-styles>"#;

        let parsed =
            read_odt_bytes(&test_package_with_content_and_styles(content, styles)).unwrap();

        assert!(parsed.sections[0].page_regions.header.read_only);
        assert!(parsed
            .warnings
            .iter()
            .any(|warning| warning.code == "odt_header_footer_unsupported"));
        assert!(matches!(
            write_odt_bytes(&parsed),
            Err(OdtError::ReadOnlyPageRegion)
        ));
    }

    #[test]
    fn externally_styled_header_span_imports_read_only_and_refuses_rewrite() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              office:version="1.3">
              <office:body><office:text><text:p>Body</text:p></office:text></office:body>
            </office:document-content>"#;
        let styles = r#"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-styles
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
              office:version="1.3">
              <office:master-styles>
                <style:master-page>
                  <style:header>
                    <text:p><text:span text:style-name="ExternalHeaderStyle">Styled header</text:span></text:p>
                  </style:header>
                </style:master-page>
              </office:master-styles>
            </office:document-styles>"#;

        let parsed =
            read_odt_bytes(&test_package_with_content_and_styles(content, styles)).unwrap();

        assert!(parsed.sections[0].page_regions.header.read_only);
        assert!(parsed
            .warnings
            .iter()
            .any(|warning| warning.code == "odt_header_footer_unsupported"));
        assert!(matches!(
            write_odt_bytes(&parsed),
            Err(OdtError::ReadOnlyPageRegion)
        ));
    }

    #[test]
    fn paragraph_style_properties_round_trip_through_odt_styles() {
        let mut document = Document::new_untitled();
        document
            .register_style(Style {
                id: StyleId::from("quote"),
                name: "Quote".to_string(),
                kind: StyleKind::Paragraph,
                parent: None,
                properties: word_core::StyleProperties {
                    paragraph: Some(ParagraphFormat {
                        alignment: Some(ParagraphAlignment::Justify),
                        line_spacing_per_mille: Some(1500),
                        spacing_before_mm: Some(2),
                        spacing_after_mm: Some(5),
                        indent_start_mm: Some(8),
                        indent_end_mm: Some(3),
                        first_line_indent_mm: Some(-4),
                    }),
                    inline: None,
                    page: None,
                },
            })
            .expect("style should register");

        let parsed =
            read_odt_bytes(&write_odt_bytes(&document).expect("write should succeed")).unwrap();
        let style = parsed
            .style(&StyleId::from("quote"))
            .expect("quote style should import");
        let format = style
            .properties
            .paragraph
            .as_ref()
            .expect("paragraph style properties should import");

        assert_eq!(format.alignment, Some(ParagraphAlignment::Justify));
        assert_eq!(format.line_spacing_per_mille, Some(1500));
        assert_eq!(format.spacing_before_mm, Some(2));
        assert_eq!(format.spacing_after_mm, Some(5));
        assert_eq!(format.indent_start_mm, Some(8));
        assert_eq!(format.indent_end_mm, Some(3));
        assert_eq!(format.first_line_indent_mm, Some(-4));
    }

    #[test]
    fn authoring_formatting_round_trips_through_generated_odt_styles() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: StyleId::from("quote"),
                format: ParagraphFormat {
                    alignment: Some(ParagraphAlignment::Justify),
                    line_spacing_per_mille: Some(1500),
                    spacing_before_mm: Some(2),
                    spacing_after_mm: Some(5),
                    indent_start_mm: Some(8),
                    indent_end_mm: None,
                    first_line_indent_mm: Some(-4),
                },
                inlines: vec![Inline {
                    text: "Formatted text".to_string(),
                    marks: vec![InlineMark::Bold],
                    link: None,
                    comment_ids: Vec::new(),
                    style: InlineStyle {
                        font_family: Some("serif".to_string()),
                        font_size_pt: Some(14),
                        text_color: Some("#1f2937".to_string()),
                        highlight_color: Some("#fff3bf".to_string()),
                    },
                    field: None,
                    note_reference: None,
                    tracked_change: None,
                }],
            }),
            Block::List(ListBlock {
                definition_id: "900w-ordered".to_string(),
                items: vec![ListItem {
                    level: 3,
                    blocks: vec![Block::Paragraph(Paragraph {
                        bookmark_id: None,
                        style: StyleId::from("body"),
                        format: Default::default(),
                        inlines: vec![Inline::text("Nested item")],
                    })],
                }],
            }),
        ];

        let parsed =
            read_odt_bytes(&write_odt_bytes(&document).expect("write should succeed")).unwrap();

        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("first block should be a paragraph");
        };
        assert_eq!(paragraph.style.as_str(), "quote");
        assert_eq!(
            paragraph.format.alignment,
            Some(ParagraphAlignment::Justify)
        );
        assert_eq!(paragraph.format.line_spacing_per_mille, Some(1500));
        assert_eq!(paragraph.format.first_line_indent_mm, Some(-4));
        assert_eq!(paragraph.inlines[0].marks, vec![InlineMark::Bold]);
        assert_eq!(
            paragraph.inlines[0].style.font_family.as_deref(),
            Some("serif")
        );
        assert_eq!(paragraph.inlines[0].style.font_size_pt, Some(14));
        assert_eq!(
            paragraph.inlines[0].style.text_color.as_deref(),
            Some("#1f2937")
        );
        assert_eq!(
            paragraph.inlines[0].style.highlight_color.as_deref(),
            Some("#fff3bf")
        );

        let Block::List(list) = &parsed.sections[0].blocks[1] else {
            panic!("second block should be a list");
        };
        assert_eq!(list.items[0].level, 3);
    }

    #[test]
    fn comments_round_trip_through_odt_annotations_with_formatting() {
        let created_at = DateTime::parse_from_rfc3339("2026-06-25T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut document = Document::new_untitled();
        document.comments.insert(
            "cmt-review".to_string(),
            CommentThread {
                id: "cmt-review".to_string(),
                author: "Local User".to_string(),
                body: "Check this wording.".to_string(),
                created_at,
                updated_at: created_at,
                resolved: true,
            },
        );
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: Default::default(),
            inlines: vec![
                Inline::text("Before "),
                Inline {
                    text: "linked bold".to_string(),
                    marks: vec![InlineMark::Bold],
                    link: Some("https://example.invalid/review".to_string()),
                    comment_ids: vec!["cmt-review".to_string()],
                    style: Default::default(),
                    field: None,
                    note_reference: None,
                    tracked_change: None,
                },
                Inline::text(" after"),
            ],
        })];

        let bytes = write_odt_bytes(&document).expect("write should succeed");
        let content_xml = content_xml_from_package(&bytes);
        assert!(content_xml.contains("<office:annotation "));
        assert!(content_xml.contains("word900:comment-id=\"cmt-review\""));
        assert!(content_xml.contains("word900:resolved=\"true\""));
        assert!(content_xml.contains("<dc:creator>Local User</dc:creator>"));
        assert!(content_xml.contains("<text:p>Check this wording.</text:p>"));
        assert!(content_xml.contains("<office:annotation-end office:name=\"cmt-review\"/>"));

        let parsed = read_odt_bytes(&bytes).expect("read should succeed");
        let comment = parsed
            .comments
            .get("cmt-review")
            .expect("comment metadata should parse");
        assert_eq!(comment.author, "Local User");
        assert_eq!(comment.body, "Check this wording.");
        assert!(comment.resolved);
        assert_eq!(comment.created_at, created_at);

        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("first block should be a paragraph");
        };
        let commented = paragraph
            .inlines
            .iter()
            .find(|inline| inline.text == "linked bold")
            .expect("commented inline should parse");
        assert_eq!(commented.marks, vec![InlineMark::Bold]);
        assert_eq!(
            commented.link.as_deref(),
            Some("https://example.invalid/review")
        );
        assert_eq!(commented.comment_ids, vec!["cmt-review"]);
    }

    #[test]
    fn notes_round_trip_through_odt_note_elements() {
        let mut document = Document::new_untitled();
        document.notes.insert(
            "note-source".to_string(),
            Note {
                id: "note-source".to_string(),
                kind: NoteKind::Footnote,
                body: "Source body".to_string(),
            },
        );
        document.notes.insert(
            "note-appendix".to_string(),
            Note {
                id: "note-appendix".to_string(),
                kind: NoteKind::Endnote,
                body: "Appendix body".to_string(),
            },
        );
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: Default::default(),
            inlines: vec![
                Inline::text("Claim"),
                Inline::note_reference(InlineNoteReference {
                    id: "note-source".to_string(),
                    kind: NoteKind::Footnote,
                    label: "1".to_string(),
                }),
                Inline::text(" appendix"),
                Inline::note_reference(InlineNoteReference {
                    id: "note-appendix".to_string(),
                    kind: NoteKind::Endnote,
                    label: "i".to_string(),
                }),
            ],
        })];

        let bytes = write_odt_bytes(&document).expect("write should succeed");
        let content_xml = content_xml_from_package(&bytes);
        assert!(content_xml.contains("<text:note "));
        assert!(content_xml.contains("text:note-class=\"footnote\""));
        assert!(content_xml.contains("text:note-class=\"endnote\""));
        assert!(content_xml.contains("word900:note-id=\"note-source\""));
        assert!(content_xml.contains("word900:note-id=\"note-appendix\""));
        assert!(content_xml.contains("<text:note-citation>1</text:note-citation>"));
        assert!(content_xml.contains("<text:note-citation>i</text:note-citation>"));
        assert!(content_xml.contains("<text:p>Source body</text:p>"));
        assert!(content_xml.contains("<text:p>Appendix body</text:p>"));

        let parsed = read_odt_bytes(&bytes).expect("read should succeed");
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        assert_eq!(parsed.notes["note-source"].body, "Source body");
        assert_eq!(parsed.notes["note-appendix"].body, "Appendix body");
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("block should remain a paragraph");
        };
        assert_eq!(
            paragraph.inlines[1].note_reference,
            Some(InlineNoteReference {
                id: "note-source".to_string(),
                kind: NoteKind::Footnote,
                label: "1".to_string(),
            })
        );
        assert_eq!(
            paragraph.inlines[3].note_reference,
            Some(InlineNoteReference {
                id: "note-appendix".to_string(),
                kind: NoteKind::Endnote,
                label: "i".to_string(),
            })
        );
    }

    #[test]
    fn external_notes_degrade_to_visible_text_with_warning() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              office:version="1.3">
              <office:body><office:text>
                <text:p>Claim<text:note text:id="ftn1" text:note-class="footnote"><text:note-citation>1</text:note-citation><text:note-body><text:p>External body</text:p></text:note-body></text:note></text:p>
              </office:text></office:body>
            </office:document-content>"#;

        let parsed = read_odt_bytes(&test_package_with_content(content)).expect("package parses");

        assert!(parsed.notes.is_empty());
        assert!(parsed
            .warnings
            .iter()
            .any(|warning| warning.code == "odt_unsupported_note"));
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("block should be a paragraph");
        };
        let text = paragraph
            .inlines
            .iter()
            .map(|inline| inline.text.as_str())
            .collect::<String>();
        assert!(text.contains("Claim[footnote 1: External body]"));
    }

    #[test]
    fn non_word900_notes_with_safe_looking_ids_degrade_to_visible_text() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              office:version="1.3">
              <office:body><office:text>
                <text:p>Claim<text:note text:id="note-smuggle" text:note-class="footnote"><text:note-citation>1</text:note-citation><text:note-body><text:p>External body</text:p></text:note-body></text:note></text:p>
              </office:text></office:body>
            </office:document-content>"#;

        let parsed = read_odt_bytes(&test_package_with_content(content)).expect("package parses");

        assert!(parsed.notes.is_empty());
        assert!(parsed
            .warnings
            .iter()
            .any(|warning| warning.code == "odt_unsupported_note"));
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("block should be a paragraph");
        };
        let text = paragraph
            .inlines
            .iter()
            .map(|inline| inline.text.as_str())
            .collect::<String>();
        assert!(text.contains("Claim[footnote 1: External body]"));
    }

    #[test]
    fn word900_notes_without_odf_kind_degrade_to_visible_text() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              xmlns:word900="https://900labs.example/ns/word"
              office:version="1.3">
              <office:body><office:text>
                <text:p>Claim<text:note text:id="note-missing-kind" word900:note-id="note-missing-kind" word900:note-kind="footnote"><text:note-citation>1</text:note-citation><text:note-body><text:p>External body</text:p></text:note-body></text:note></text:p>
              </office:text></office:body>
            </office:document-content>"#;

        let parsed = read_odt_bytes(&test_package_with_content(content)).expect("package parses");

        assert!(parsed.notes.is_empty());
        assert!(parsed
            .warnings
            .iter()
            .any(|warning| warning.code == "odt_unsupported_note"));
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("block should be a paragraph");
        };
        let text = paragraph
            .inlines
            .iter()
            .map(|inline| inline.text.as_str())
            .collect::<String>();
        assert!(text.contains("Claim[footnote 1: External body]"));
    }

    #[test]
    fn oversized_word900_note_degrades_to_visible_text_with_warning() {
        let oversized_body = "x".repeat(word_core::MAX_NOTE_BODY_CHARS + 1);
        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              xmlns:word900="https://900labs.example/ns/word"
              office:version="1.3">
              <office:body><office:text>
                <text:p>Claim<text:note text:id="note-overflow" text:note-class="footnote" word900:note-id="note-overflow" word900:note-kind="footnote"><text:note-citation>1</text:note-citation><text:note-body><text:p>{oversized_body}</text:p></text:note-body></text:note></text:p>
              </office:text></office:body>
            </office:document-content>"#
        );

        let parsed = read_odt_bytes(&test_package_with_content(&content)).expect("package parses");

        assert!(parsed.notes.is_empty());
        assert!(parsed
            .warnings
            .iter()
            .any(|warning| warning.code == "odt_invalid_note"));
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("block should be a paragraph");
        };
        let text = paragraph
            .inlines
            .iter()
            .map(|inline| inline.text.as_str())
            .collect::<String>();
        assert!(text.starts_with("Claim[footnote 1: "));
    }

    #[test]
    fn excess_word900_notes_degrade_to_visible_text_with_warning() {
        let mut notes = String::new();
        for index in 0..=word_core::MAX_NOTES {
            notes.push_str(&format!(
                r#"<text:note text:id="note-{index}" text:note-class="footnote" word900:note-id="note-{index}" word900:note-kind="footnote"><text:note-citation>{}</text:note-citation><text:note-body><text:p>Body {index}</text:p></text:note-body></text:note>"#,
                index + 1
            ));
        }
        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              xmlns:word900="https://900labs.example/ns/word"
              office:version="1.3">
              <office:body><office:text>
                <text:p>Claim{notes}</text:p>
              </office:text></office:body>
            </office:document-content>"#
        );

        let parsed = read_odt_bytes(&test_package_with_content(&content)).expect("package parses");

        assert_eq!(parsed.notes.len(), word_core::MAX_NOTES);
        assert!(parsed
            .warnings
            .iter()
            .any(|warning| warning.code == "odt_too_many_notes"));
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("block should be a paragraph");
        };
        let text = paragraph
            .inlines
            .iter()
            .map(|inline| inline.text.as_str())
            .collect::<String>();
        assert!(text.contains("[footnote 513: Body 512]"));
    }

    #[test]
    fn tracked_changes_round_trip_through_word900_odt_metadata() {
        let created_at = DateTime::parse_from_rfc3339("2026-06-25T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut document = Document::new_untitled();
        document.track_changes.recording = true;
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: Default::default(),
            inlines: vec![
                Inline::text("Before "),
                Inline {
                    text: "inserted".to_string(),
                    marks: vec![InlineMark::Underline],
                    link: Some("https://example.invalid/change".to_string()),
                    comment_ids: Vec::new(),
                    style: Default::default(),
                    field: None,
                    note_reference: None,
                    tracked_change: Some(TrackedChange {
                        id: "chg-insert".to_string(),
                        kind: TrackedChangeKind::Insertion,
                        author: "Local User".to_string(),
                        created_at,
                    }),
                },
                Inline {
                    text: " deleted".to_string(),
                    marks: vec![InlineMark::Strikethrough],
                    link: None,
                    comment_ids: Vec::new(),
                    style: Default::default(),
                    field: None,
                    note_reference: None,
                    tracked_change: Some(TrackedChange {
                        id: "chg-delete".to_string(),
                        kind: TrackedChangeKind::Deletion,
                        author: "Local User".to_string(),
                        created_at,
                    }),
                },
            ],
        })];

        let bytes = write_odt_bytes(&document).expect("write should succeed");
        let content_xml = content_xml_from_package(&bytes);
        assert!(content_xml.contains("word900:track-changes-recording=\"true\""));
        assert!(content_xml.contains("word900:change-id=\"chg-insert\""));
        assert!(content_xml.contains("word900:change-kind=\"insertion\""));
        assert!(content_xml.contains("word900:change-id=\"chg-delete\""));
        assert!(content_xml.contains("word900:change-kind=\"deletion\""));
        assert!(content_xml.contains("word900:change-author=\"Local User\""));

        let parsed = read_odt_bytes(&bytes).expect("read should succeed");
        assert!(parsed.track_changes.recording);
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("first block should be a paragraph");
        };
        let inserted = paragraph
            .inlines
            .iter()
            .find(|inline| inline.text == "inserted")
            .expect("inserted inline should parse");
        let inserted_change = inserted
            .tracked_change
            .as_ref()
            .expect("inserted change metadata should parse");
        assert_eq!(inserted_change.kind, TrackedChangeKind::Insertion);
        assert_eq!(inserted_change.author, "Local User");
        assert_eq!(inserted_change.created_at, created_at);
        assert_eq!(
            inserted.link.as_deref(),
            Some("https://example.invalid/change")
        );
        assert_eq!(inserted.marks, vec![InlineMark::Underline]);

        let deleted = paragraph
            .inlines
            .iter()
            .find(|inline| inline.text == " deleted")
            .expect("deleted inline should parse");
        assert_eq!(
            deleted.tracked_change.as_ref().map(|change| change.kind),
            Some(TrackedChangeKind::Deletion)
        );
        assert_eq!(deleted.marks, vec![InlineMark::Strikethrough]);
    }

    #[test]
    fn non_word900_tracked_change_attributes_are_ignored() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:evil="urn:example:evil"
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              office:version="1.3">
              <office:body>
                <office:text evil:track-changes-recording="true">
                  <text:p>Before <text:span evil:change-id="chg-evil" evil:change-kind="deletion" evil:change-author="External User" evil:change-created-at="2026-06-25T12:00:00Z">keep</text:span></text:p>
                </office:text>
              </office:body>
            </office:document-content>"#;

        let parsed = read_odt_bytes(&test_package_with_content(content)).expect("package parses");

        assert!(!parsed.track_changes.recording);
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("first block should be a paragraph");
        };
        assert!(paragraph
            .inlines
            .iter()
            .all(|inline| inline.tracked_change.is_none()));
    }

    #[test]
    fn malformed_word900_tracked_change_metadata_is_ignored() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              xmlns:word900="urn:900labs:900word:metadata"
              office:version="1.3">
              <office:body>
                <office:text word900:track-changes-recording="true">
                  <text:p>Before <text:span word900:change-id="chg-bad" word900:change-kind="deletion" word900:change-created-at="not-a-date">keep</text:span></text:p>
                </office:text>
              </office:body>
            </office:document-content>"#;

        let parsed = read_odt_bytes(&test_package_with_content(content)).expect("package parses");

        assert!(parsed.track_changes.recording);
        assert!(parsed
            .warnings
            .iter()
            .any(|warning| warning.code == "odt_unsafe_tracked_change"));
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("first block should be a paragraph");
        };
        assert!(paragraph
            .inlines
            .iter()
            .all(|inline| inline.tracked_change.is_none()));
    }

    #[test]
    fn unanchored_imported_comment_metadata_is_pruned() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:dc="http://purl.org/dc/elements/1.1/"
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              xmlns:word900="urn:900word:metadata:1.0"
              office:version="1.3">
              <office:body><office:text><text:p>Before<office:annotation office:name="cmt-point" word900:comment-id="cmt-point" word900:resolved="false"><dc:creator>Local User</dc:creator><dc:date>2026-06-25T12:00:00Z</dc:date><text:p>Point-only note</text:p></office:annotation><office:annotation-end office:name="cmt-point"/>After</text:p></office:text></office:body>
            </office:document-content>"#;

        let parsed = read_odt_bytes(&test_package_with_content(content)).expect("package parses");

        assert!(parsed.comments.is_empty());
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("first block should be a paragraph");
        };
        assert_eq!(
            paragraph
                .inlines
                .iter()
                .map(|inline| inline.text.as_str())
                .collect::<String>(),
            "BeforeAfter"
        );
        assert!(paragraph
            .inlines
            .iter()
            .all(|inline| inline.comment_ids.is_empty()));
    }

    #[test]
    fn bookmarks_and_internal_links_round_trip_through_odt() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::Heading(Heading {
                bookmark_id: Some("bm-heading".to_string()),
                level: 2,
                inlines: vec![Inline::text("Target")],
            }),
            Block::Paragraph(Paragraph {
                bookmark_id: Some("bm-body".to_string()),
                style: StyleId::from("body"),
                format: Default::default(),
                inlines: vec![Inline {
                    text: "Jump".to_string(),
                    marks: Vec::new(),
                    link: Some("#bm-heading".to_string()),
                    comment_ids: Vec::new(),
                    style: Default::default(),
                    field: None,
                    note_reference: None,
                    tracked_change: None,
                }],
            }),
        ];

        let bytes = write_odt_bytes(&document).expect("write should succeed");
        let content_xml = content_xml_from_package(&bytes);
        assert!(content_xml.contains(r#"<text:bookmark text:name="bm-heading"/>"#));
        assert!(content_xml.contains(r#"<text:bookmark text:name="bm-body"/>"#));
        let parsed = read_odt_bytes(&bytes).expect("read should succeed");

        let Block::Heading(heading) = &parsed.sections[0].blocks[0] else {
            panic!("first block should be a heading");
        };
        assert_eq!(heading.bookmark_id.as_deref(), Some("bm-heading"));
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[1] else {
            panic!("second block should be a paragraph");
        };
        assert_eq!(paragraph.bookmark_id.as_deref(), Some("bm-body"));
        assert_eq!(paragraph.inlines[0].link.as_deref(), Some("#bm-heading"));
    }

    #[test]
    fn table_of_contents_round_trips_through_word900_metadata() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::TableOfContents(TableOfContents {
                title: "Contents".to_string(),
                entries: vec![
                    TableOfContentsEntry {
                        level: 1,
                        text: "Intro".to_string(),
                        target_bookmark_id: "bm-intro".to_string(),
                    },
                    TableOfContentsEntry {
                        level: 3,
                        text: "Details".to_string(),
                        target_bookmark_id: "bm-details".to_string(),
                    },
                ],
            }),
            Block::Heading(Heading {
                bookmark_id: Some("bm-intro".to_string()),
                level: 1,
                inlines: vec![Inline::text("Intro")],
            }),
            Block::Heading(Heading {
                bookmark_id: Some("bm-details".to_string()),
                level: 3,
                inlines: vec![Inline::text("Details")],
            }),
        ];

        let bytes = write_odt_bytes(&document).expect("write should succeed");
        let content_xml = content_xml_from_package(&bytes);
        assert!(content_xml.contains("word900:block-type=\"table-of-contents\""));
        assert!(content_xml.contains("word900:toc-entries="));
        assert!(content_xml.contains("xlink:href=\"#bm-intro\""));
        assert!(!content_xml.contains("<text:page-number"));
        assert!(!content_xml.contains("<text:page-count"));

        let parsed = read_odt_bytes(&bytes).expect("read should succeed");
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        let Block::TableOfContents(table_of_contents) = &parsed.sections[0].blocks[0] else {
            panic!("first block should remain a toc");
        };
        assert_eq!(table_of_contents.title, "Contents");
        assert_eq!(
            table_of_contents.entries,
            vec![
                TableOfContentsEntry {
                    level: 1,
                    text: "Intro".to_string(),
                    target_bookmark_id: "bm-intro".to_string(),
                },
                TableOfContentsEntry {
                    level: 3,
                    text: "Details".to_string(),
                    target_bookmark_id: "bm-details".to_string(),
                },
            ]
        );
    }

    #[test]
    fn table_cell_presentation_round_trips_through_word900_metadata() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Table(Table {
            rows: vec![TableRow {
                cells: vec![TableCell {
                    presentation: TableCellPresentation {
                        background_color: Some("#fff3bf".to_string()),
                        text_alignment: Some(ParagraphAlignment::Center),
                        border: TableCellBorder::Hidden,
                    },
                    blocks: vec![Block::Paragraph(Paragraph {
                        bookmark_id: None,
                        style: StyleId::from("body"),
                        format: Default::default(),
                        inlines: vec![Inline::text("Styled cell")],
                    })],
                }],
            }],
        })];

        let bytes = write_odt_bytes(&document).expect("odt should write");
        let content_xml = content_xml_from_package(&bytes);
        assert!(content_xml.contains("word900:cell-background-color=\"#fff3bf\""));
        assert!(content_xml.contains("word900:cell-text-align=\"center\""));
        assert!(content_xml.contains("word900:cell-border=\"hidden\""));

        let parsed = read_odt_bytes(&bytes).expect("odt should read");
        let Block::Table(table) = &parsed.sections[0].blocks[0] else {
            panic!("first block should be a table");
        };
        let presentation = &table.rows[0].cells[0].presentation;
        assert_eq!(presentation.background_color.as_deref(), Some("#fff3bf"));
        assert_eq!(
            presentation.text_alignment,
            Some(ParagraphAlignment::Center)
        );
        assert_eq!(presentation.border, TableCellBorder::Hidden);
    }

    #[test]
    fn table_cell_presentation_omits_unrecognized_background_color() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Table(Table {
            rows: vec![TableRow {
                cells: vec![TableCell {
                    presentation: TableCellPresentation {
                        background_color: Some("rgb(1, 2, 3)".to_string()),
                        text_alignment: None,
                        border: TableCellBorder::Visible,
                    },
                    blocks: vec![Block::Paragraph(Paragraph {
                        bookmark_id: None,
                        style: StyleId::from("body"),
                        format: Default::default(),
                        inlines: vec![Inline::text("Cell")],
                    })],
                }],
            }],
        })];

        let bytes = write_odt_bytes(&document).expect("odt should write");
        let content_xml = content_xml_from_package(&bytes);
        assert!(!content_xml.contains("cell-background-color"));
        assert!(!content_xml.contains("rgb(1"));
    }

    #[test]
    fn invalid_external_table_cell_presentation_attrs_are_ignored() {
        let content = r##"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              xmlns:word900="urn:900labs:900word:metadata"
              office:version="1.3">
              <office:body><office:text>
                <table:table>
                  <table:table-row>
                    <table:table-cell
                      word900:cell-background-color="#ff00ff"
                      word900:cell-text-align="diagonal"
                      word900:cell-border="double">
                      <text:p>External cell</text:p>
                    </table:table-cell>
                  </table:table-row>
                </table:table>
              </office:text></office:body>
            </office:document-content>"##;

        let parsed = read_odt_bytes(&test_package_with_content(content)).expect("package parses");

        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        let Block::Table(table) = &parsed.sections[0].blocks[0] else {
            panic!("first block should be a table");
        };
        let presentation = &table.rows[0].cells[0].presentation;
        assert!(presentation.is_default());
        assert_eq!(presentation.background_color, None);
        assert_eq!(presentation.text_alignment, None);
        assert_eq!(presentation.border, TableCellBorder::Visible);
    }

    #[test]
    fn mismatched_table_of_contents_metadata_imports_as_visible_text() {
        let hidden_entries =
            escape_xml(r#"[{"level":1,"text":"Hidden payload","target_bookmark_id":"bm-hidden"}]"#);
        let content = format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              xmlns:xlink="http://www.w3.org/1999/xlink"
              xmlns:word900="urn:900labs:900word:metadata"
              office:version="1.3">
              <office:body><office:text>
                <text:p text:style-name="900w-toc" word900:block-type="table-of-contents" word900:toc-title="Contents" word900:toc-entries="{hidden_entries}">
                  Contents<text:line-break/><text:a xlink:href="#bm-visible">Visible heading</text:a>
                </text:p>
              </office:text></office:body>
            </office:document-content>"##
        );

        let parsed = read_odt_bytes(&test_package_with_content(&content)).expect("package parses");

        assert!(parsed
            .warnings
            .iter()
            .any(|warning| warning.code == "odt_unsupported_toc_metadata"));
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("mismatched toc should import as a paragraph");
        };
        let visible_text = paragraph
            .inlines
            .iter()
            .map(|inline| inline.text.as_str())
            .collect::<String>();
        assert!(visible_text.contains("Visible heading"));
        assert!(!visible_text.contains("Hidden payload"));
    }

    #[test]
    fn oversized_table_of_contents_metadata_imports_as_visible_text() {
        let oversized_entries = escape_xml(&format!(
            r#"[{{"level":1,"text":"{}","target_bookmark_id":"bm-visible"}}]"#,
            "A".repeat(17 * 1024)
        ));
        let content = format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              xmlns:xlink="http://www.w3.org/1999/xlink"
              xmlns:word900="urn:900labs:900word:metadata"
              office:version="1.3">
              <office:body><office:text>
                <text:p text:style-name="900w-toc" word900:block-type="table-of-contents" word900:toc-title="Contents" word900:toc-entries="{oversized_entries}">
                  Contents<text:line-break/><text:a xlink:href="#bm-visible">Visible heading</text:a>
                </text:p>
              </office:text></office:body>
            </office:document-content>"##
        );

        let parsed = read_odt_bytes(&test_package_with_content(&content)).expect("package parses");

        assert!(parsed
            .warnings
            .iter()
            .any(|warning| warning.code == "odt_unsupported_toc_metadata"));
        assert!(matches!(&parsed.sections[0].blocks[0], Block::Paragraph(_)));
    }

    #[test]
    fn unsafe_imported_bookmarks_and_internal_links_are_stripped() {
        let content = r##"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              xmlns:xlink="http://www.w3.org/1999/xlink"
              office:version="1.3">
              <office:body><office:text>
                <text:h text:outline-level="1"><text:bookmark text:name="../bad"/><text:a xlink:href="#../bad">Unsafe</text:a></text:h>
                <text:p><text:bookmark text:name="bm-good"/><text:a xlink:href="#bm-good">Safe</text:a></text:p>
              </office:text></office:body>
            </office:document-content>"##;

        let parsed = read_odt_bytes(&test_package_with_content(content)).expect("package parses");

        let Block::Heading(heading) = &parsed.sections[0].blocks[0] else {
            panic!("first block should be a heading");
        };
        assert_eq!(heading.bookmark_id, None);
        assert_eq!(heading.inlines[0].link, None);
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[1] else {
            panic!("second block should be a paragraph");
        };
        assert_eq!(paragraph.bookmark_id.as_deref(), Some("bm-good"));
        assert_eq!(paragraph.inlines[0].link.as_deref(), Some("#bm-good"));
        assert!(parsed
            .warnings
            .iter()
            .any(|warning| warning.code == "odt_unsafe_bookmark"));
        assert!(parsed
            .warnings
            .iter()
            .any(|warning| warning.code == "odt_unsafe_link"));
    }

    #[test]
    fn parsed_documents_keep_default_list_definitions_for_new_lists() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              office:version="1.3">
              <office:body><office:text><text:p>Imported body</text:p></office:text></office:body>
            </office:document-content>"#;
        let mut document =
            read_odt_bytes(&test_package_with_content(content)).expect("package parses");
        document.sections[0].blocks.push(Block::List(ListBlock {
            definition_id: "900w-ordered".to_string(),
            items: vec![ListItem {
                level: 1,
                blocks: vec![Block::Paragraph(Paragraph {
                    bookmark_id: None,
                    style: StyleId::from("body"),
                    format: Default::default(),
                    inlines: vec![Inline::text("Numbered")],
                })],
            }],
        }));

        let parsed =
            read_odt_bytes(&write_odt_bytes(&document).expect("rewrite should succeed")).unwrap();

        let Block::List(list) = parsed.sections[0].blocks.last().expect("list should exist") else {
            panic!("last block should be a list");
        };
        assert_eq!(
            parsed
                .lists
                .get(&list.definition_id)
                .map(|definition| definition.ordered),
            Some(true)
        );
    }

    #[test]
    fn remote_image_reference_imports_with_warning() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
            <office:document-content
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
              xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
              xmlns:xlink="http://www.w3.org/1999/xlink"
              office:version="1.3">
              <office:body><office:text>
                <text:p text:style-name="900w-image">
                  <draw:frame draw:name="remote.png">
                    <draw:image xlink:href="https://example.invalid/remote.png"/>
                  </draw:frame>
                </text:p>
              </office:text></office:body>
            </office:document-content>"#;
        let parsed = read_odt_bytes(&test_package_with_content(content)).expect("package parses");

        assert!(parsed.sections[0].blocks.is_empty());
        assert!(parsed
            .warnings
            .iter()
            .any(|warning| warning.code == "odt_unsafe_image_href"));
    }

    fn sample_document() -> Document {
        let mut document = Document::new_untitled();
        document.meta.title = "ODT MVP Sample".to_string();
        document
            .register_style(Style {
                id: StyleId::from("caption"),
                name: "Caption".to_string(),
                kind: StyleKind::Paragraph,
                parent: None,
                properties: Default::default(),
            })
            .expect("style should register");
        document.assets.insert(
            "sample.png".to_string(),
            AssetRef {
                id: "sample.png".to_string(),
                media_type: "image/png".to_string(),
                byte_len: SAMPLE_PNG.len(),
                bytes: SAMPLE_PNG.to_vec(),
                original_name: Some("sample.png".to_string()),
            },
        );
        document.lists.insert(
            "tasks".to_string(),
            ListDefinition {
                ordered: false,
                marker: None,
            },
        );
        document.sections[0].blocks = vec![
            Block::Heading(Heading {
                bookmark_id: None,
                level: 1,
                inlines: vec![Inline::text("Sprint 003")],
            }),
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: StyleId::from("caption"),
                format: Default::default(),
                inlines: vec![
                    Inline {
                        text: "Bold العربية 中文 ".to_string(),
                        marks: vec![InlineMark::Bold],
                        link: None,
                        comment_ids: Vec::new(),
                        style: Default::default(),
                        field: None,
                        note_reference: None,
                        tracked_change: None,
                    },
                    Inline {
                        text: "linked text".to_string(),
                        marks: vec![InlineMark::Italic, InlineMark::Underline],
                        link: Some("https://example.invalid/reference".to_string()),
                        comment_ids: Vec::new(),
                        style: Default::default(),
                        field: None,
                        note_reference: None,
                        tracked_change: None,
                    },
                ],
            }),
            Block::List(ListBlock {
                definition_id: "tasks".to_string(),
                items: vec![
                    ListItem {
                        level: 1,
                        blocks: vec![Block::Paragraph(Paragraph {
                            bookmark_id: None,
                            style: StyleId::from("body"),
                            format: Default::default(),
                            inlines: vec![Inline::text("First item")],
                        })],
                    },
                    ListItem {
                        level: 1,
                        blocks: vec![Block::Paragraph(Paragraph {
                            bookmark_id: None,
                            style: StyleId::from("body"),
                            format: Default::default(),
                            inlines: vec![Inline::text("Second item")],
                        })],
                    },
                ],
            }),
            Block::Table(Table {
                rows: vec![
                    TableRow {
                        cells: vec![
                            TableCell {
                                presentation: Default::default(),
                                blocks: vec![Block::Paragraph(Paragraph {
                                    bookmark_id: None,
                                    style: StyleId::from("body"),
                                    format: Default::default(),
                                    inlines: vec![Inline::text("A1")],
                                })],
                            },
                            TableCell {
                                presentation: Default::default(),
                                blocks: vec![Block::Paragraph(Paragraph {
                                    bookmark_id: None,
                                    style: StyleId::from("body"),
                                    format: Default::default(),
                                    inlines: vec![Inline::text("B1")],
                                })],
                            },
                        ],
                    },
                    TableRow {
                        cells: vec![TableCell {
                            presentation: Default::default(),
                            blocks: vec![Block::Paragraph(Paragraph {
                                bookmark_id: None,
                                style: StyleId::from("body"),
                                format: Default::default(),
                                inlines: vec![Inline::text("A2")],
                            })],
                        }],
                    },
                ],
            }),
            Block::Image(ImageBlock {
                asset_id: "sample.png".to_string(),
                presentation: ImagePresentation {
                    alignment: ImageAlignment::Center,
                    scale_percent: 75,
                    caption: Some("Synthetic caption".to_string()),
                },
                alt_text: Some("Synthetic sample image".to_string()),
            }),
        ];
        document
    }

    fn test_package_with_content(content: &str) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        writer
            .start_file(
                "mimetype",
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(ODT_MIME_TYPE.as_bytes()).unwrap();
        writer
            .start_file(
                "content.xml",
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(content.as_bytes()).unwrap();
        writer.finish().unwrap().into_inner()
    }

    fn content_xml_from_package(bytes: &[u8]) -> String {
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor).unwrap();
        let mut content = String::new();
        archive
            .by_name("content.xml")
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        content
    }

    fn test_package_with_content_and_styles(content: &str, styles: &str) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        writer
            .start_file(
                "mimetype",
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(ODT_MIME_TYPE.as_bytes()).unwrap();
        writer
            .start_file(
                "content.xml",
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(content.as_bytes()).unwrap();
        writer
            .start_file(
                "styles.xml",
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(styles.as_bytes()).unwrap();
        writer.finish().unwrap().into_inner()
    }

    fn test_package_without_mimetype(content: &str) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        writer
            .start_file(
                "content.xml",
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(content.as_bytes()).unwrap();
        writer.finish().unwrap().into_inner()
    }

    fn test_package_with_image(image: Vec<u8>) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        writer
            .start_file(
                "mimetype",
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(ODT_MIME_TYPE.as_bytes()).unwrap();
        writer
            .start_file(
                "content.xml",
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
        writer
            .write_all(
                br#"<?xml version="1.0"?><office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"/>"#,
            )
            .unwrap();
        writer
            .start_file(
                "Pictures/image.png",
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(&image).unwrap();
        writer.finish().unwrap().into_inner()
    }
}
