use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};
use thiserror::Error;
use word_core::{
    sanitize_bookmark_id, Block, Document, DocumentWarning, Heading, Inline, InlineMark, ListBlock,
    ListItem, Paragraph, StyleId, Table, TableCell, TableRow,
};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const DOCUMENT_XML: &str = "word/document.xml";
const DOCUMENT_RELS: &str = "word/_rels/document.xml.rels";
const NUMBERING_XML: &str = "word/numbering.xml";
const REL_TYPE_HYPERLINK: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";
const REL_TYPE_OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
const REL_TYPE_STYLES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";
const REL_TYPE_NUMBERING: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackageLimits {
    pub max_package_size: u64,
    pub max_entries: usize,
    pub max_entry_size: u64,
    pub max_total_expanded_size: u64,
    pub max_path_depth: usize,
    pub max_xml_depth: usize,
}

impl Default for PackageLimits {
    fn default() -> Self {
        Self {
            max_package_size: 64 * 1024 * 1024,
            max_entries: 384,
            max_entry_size: 8 * 1024 * 1024,
            max_total_expanded_size: 32 * 1024 * 1024,
            max_path_depth: 10,
            max_xml_depth: 160,
        }
    }
}

#[derive(Debug, Error)]
pub enum DocxError {
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
    #[error("package expanded size is too large")]
    ExpandedSizeTooLarge,
    #[error("package path is too deep: {name}")]
    PathTooDeep { name: String },
    #[error("unsafe package path: {name}")]
    UnsafePath { name: String },
    #[error("symlink package entry is not allowed: {name}")]
    SymlinkEntry { name: String },
    #[error("encrypted package entry is not allowed: {name}")]
    EncryptedEntry { name: String },
    #[error("unexpected executable package entry: {name}")]
    ExecutableEntry { name: String },
    #[error("missing DOCX document.xml")]
    MissingDocument,
    #[error("xml error in {name}: {message}")]
    Xml { name: String, message: String },
    #[error("xml depth exceeds limit in {name}")]
    XmlTooDeep { name: String },
    #[error("xml entity declarations are not allowed in {name}")]
    XmlEntityDeclaration { name: String },
}

#[derive(Debug, Default)]
struct WarningSink {
    warnings: Vec<DocumentWarning>,
    seen: BTreeSet<String>,
}

impl WarningSink {
    fn warn(&mut self, code: &'static str, message: &'static str) {
        if self.seen.insert(code.to_string()) {
            self.warnings.push(DocumentWarning {
                code: code.to_string(),
                message: message.to_string(),
            });
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ListMarker {
    ordered: bool,
    level: u8,
}

#[derive(Debug, Clone)]
struct ParsedBlock {
    block: Block,
    list_marker: Option<ListMarker>,
}

#[derive(Debug, Clone, Default)]
struct RelationshipMap {
    hyperlinks: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default)]
struct NumberingMap {
    abstract_ordered: BTreeMap<String, bool>,
    num_to_abstract: BTreeMap<String, String>,
}

impl NumberingMap {
    fn ordered_for_num_id(&self, num_id: &str) -> Option<bool> {
        self.num_to_abstract
            .get(num_id)
            .and_then(|abstract_id| self.abstract_ordered.get(abstract_id))
            .copied()
    }
}

#[derive(Debug, Clone, Default)]
struct ParagraphProperties {
    heading_level: Option<u8>,
    list_marker: Option<ListMarker>,
}

#[derive(Debug, Clone, Default)]
struct NumberingProperties {
    num_id: Option<String>,
    level: Option<u8>,
}

#[derive(Debug, Clone, Default)]
struct RunProperties {
    bold: bool,
    italic: bool,
    underline: bool,
}

impl RunProperties {
    fn marks(&self) -> Vec<InlineMark> {
        let mut marks = Vec::new();
        if self.bold {
            marks.push(InlineMark::Bold);
        }
        if self.italic {
            marks.push(InlineMark::Italic);
        }
        if self.underline {
            marks.push(InlineMark::Underline);
        }
        marks
    }
}

#[derive(Debug, Clone)]
struct HyperlinkRef {
    href: Option<String>,
}

#[derive(Debug, Clone)]
struct HyperlinkIds {
    external: BTreeMap<String, String>,
}

pub fn read_docx_bytes(bytes: &[u8]) -> Result<Document, DocxError> {
    read_docx_bytes_with_limits(bytes, PackageLimits::default())
}

pub fn read_docx_bytes_with_limits(
    bytes: &[u8],
    limits: PackageLimits,
) -> Result<Document, DocxError> {
    validate_docx_package(bytes, limits)?;

    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;
    let mut document_xml = String::new();
    let mut rels_xml = String::new();
    let mut numbering_xml = String::new();

    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        if file.is_dir() {
            continue;
        }
        match file.name() {
            DOCUMENT_XML => {
                file.read_to_string(&mut document_xml)?;
            }
            DOCUMENT_RELS => {
                file.read_to_string(&mut rels_xml)?;
            }
            NUMBERING_XML => {
                file.read_to_string(&mut numbering_xml)?;
            }
            _ => {}
        }
    }

    if document_xml.is_empty() {
        return Err(DocxError::MissingDocument);
    }

    let mut warnings = WarningSink::default();
    let rels = if rels_xml.is_empty() {
        RelationshipMap::default()
    } else {
        parse_relationships_xml(&rels_xml, &mut warnings)?
    };
    let numbering = if numbering_xml.is_empty() {
        NumberingMap::default()
    } else {
        parse_numbering_xml(&numbering_xml, &mut warnings)?
    };
    let blocks = parse_document_xml(&document_xml, &rels, &numbering, &mut warnings)?;

    let mut document = Document::new_untitled();
    if let Some(section) = document.sections.first_mut() {
        section.blocks = if blocks.is_empty() {
            vec![empty_paragraph_block()]
        } else {
            blocks
        };
    }
    document.warnings = warnings.warnings;
    Ok(document)
}

pub fn write_docx_bytes(document: &Document) -> Result<Vec<u8>, DocxError> {
    let hyperlinks = collect_external_hyperlinks(document);
    let hyperlink_ids = assign_hyperlink_ids(&hyperlinks);
    let compressed = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);

    writer.start_file("[Content_Types].xml", compressed)?;
    writer.write_all(render_content_types_xml().as_bytes())?;

    writer.start_file("_rels/.rels", compressed)?;
    writer.write_all(render_root_rels_xml().as_bytes())?;

    writer.start_file(DOCUMENT_XML, compressed)?;
    writer.write_all(render_document_xml(document, &hyperlink_ids).as_bytes())?;

    writer.start_file(DOCUMENT_RELS, compressed)?;
    writer.write_all(render_document_rels_xml(&hyperlink_ids).as_bytes())?;

    writer.start_file("word/styles.xml", compressed)?;
    writer.write_all(render_styles_xml().as_bytes())?;

    writer.start_file(NUMBERING_XML, compressed)?;
    writer.write_all(render_numbering_xml().as_bytes())?;

    Ok(writer.finish()?.into_inner())
}

pub fn validate_docx_package(bytes: &[u8], limits: PackageLimits) -> Result<(), DocxError> {
    if bytes.len() as u64 > limits.max_package_size {
        return Err(DocxError::PackageTooLarge);
    }

    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;
    let entry_count = archive.len();
    if entry_count > limits.max_entries {
        return Err(DocxError::TooManyEntries { count: entry_count });
    }

    let mut expanded_size = 0_u64;
    let mut has_document = false;

    for index in 0..entry_count {
        let mut file = archive.by_index(index)?;
        let name = file.name().to_string();
        validate_entry_path(&name, limits)?;
        validate_entry_mode(&file, &name)?;
        validate_entry_kind(&name)?;
        if file.is_dir() {
            continue;
        }
        if file.size() > limits.max_entry_size {
            return Err(DocxError::EntryTooLarge { name });
        }
        expanded_size = expanded_size.saturating_add(file.size());
        if expanded_size > limits.max_total_expanded_size {
            return Err(DocxError::ExpandedSizeTooLarge);
        }
        if name == DOCUMENT_XML {
            has_document = true;
        }
        if name.ends_with(".xml") || name.ends_with(".rels") {
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            validate_xml_preflight(&name, &content, limits)?;
        }
    }

    if !has_document {
        return Err(DocxError::MissingDocument);
    }
    Ok(())
}

fn parse_relationships_xml(
    xml: &str,
    warnings: &mut WarningSink,
) -> Result<RelationshipMap, DocxError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut relationships = RelationshipMap::default();

    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_RELS, err))?
        {
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"Relationship" =>
            {
                let id = attr_value(&start, b"Id", DOCUMENT_RELS)?;
                let rel_type = attr_value(&start, b"Type", DOCUMENT_RELS)?;
                let target = attr_value(&start, b"Target", DOCUMENT_RELS)?;
                let target_mode = attr_value(&start, b"TargetMode", DOCUMENT_RELS)?;
                match (id, rel_type, target) {
                    (Some(id), Some(rel_type), Some(target)) if rel_type == REL_TYPE_HYPERLINK => {
                        if target_mode.as_deref() != Some("External") {
                            warnings.warn(
                                "docx_internal_hyperlink_ignored",
                                "Unsupported internal DOCX hyperlinks were imported as plain text",
                            );
                            continue;
                        }
                        if let Some(href) = sanitize_text_href(&target) {
                            relationships.hyperlinks.insert(id, href);
                        } else {
                            warnings.warn(
                                "docx_unsafe_hyperlink",
                                "Unsafe DOCX hyperlinks were stripped during import",
                            );
                        }
                    }
                    (_, Some(_), _) if target_mode.as_deref() == Some("External") => {
                        warnings.warn(
                            "docx_external_relationship_ignored",
                            "Unsupported external DOCX relationships were ignored during import",
                        );
                    }
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(relationships)
}

fn parse_numbering_xml(xml: &str, warnings: &mut WarningSink) -> Result<NumberingMap, DocxError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut numbering = NumberingMap::default();
    let mut current_abstract: Option<String> = None;
    let mut current_num: Option<String> = None;

    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(NUMBERING_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"abstractNum" => {
                current_abstract = attr_value(&start, b"abstractNumId", NUMBERING_XML)?;
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"abstractNum" => {
                current_abstract = None;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"num" => {
                current_num = attr_value(&start, b"numId", NUMBERING_XML)?;
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"num" => {
                current_num = None;
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"numFmt" =>
            {
                if let Some(abstract_id) = current_abstract.as_ref() {
                    let value = attr_value(&start, b"val", NUMBERING_XML)?
                        .unwrap_or_else(|| "decimal".to_string());
                    numbering
                        .abstract_ordered
                        .insert(abstract_id.clone(), value != "bullet");
                }
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"abstractNumId" =>
            {
                if let Some(num_id) = current_num.as_ref() {
                    if let Some(abstract_id) = attr_value(&start, b"val", NUMBERING_XML)? {
                        numbering
                            .num_to_abstract
                            .insert(num_id.clone(), abstract_id);
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    if numbering.num_to_abstract.is_empty() && !xml.trim().is_empty() {
        warnings.warn(
            "docx_numbering_part_degraded",
            "Unsupported DOCX numbering details were imported with generic list markers",
        );
    }

    Ok(numbering)
}

fn parse_document_xml(
    xml: &str,
    rels: &RelationshipMap,
    numbering: &NumberingMap,
    warnings: &mut WarningSink,
) -> Result<Vec<Block>, DocxError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut parsed = Vec::new();
    let mut in_body = false;

    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"body" => {
                in_body = true;
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"body" => break,
            Event::Start(start) if in_body && local_name(start.name().as_ref()) == b"p" => {
                let block = parse_paragraph(&mut reader, rels, numbering, warnings)?;
                push_parsed_block(&mut parsed, block);
            }
            Event::Start(start) if in_body && local_name(start.name().as_ref()) == b"tbl" => {
                let table = parse_table(&mut reader, rels, numbering, warnings)?;
                push_parsed_block(
                    &mut parsed,
                    ParsedBlock {
                        block: Block::Table(table),
                        list_marker: None,
                    },
                );
            }
            Event::Start(start) if in_body && local_name(start.name().as_ref()) == b"sectPr" => {
                skip_element(&mut reader, b"sectPr", DOCUMENT_XML)?;
            }
            Event::Empty(_) if in_body => {
                warnings.warn(
                    "docx_unsupported_body_content",
                    "Unsupported DOCX body content was ignored during import",
                );
            }
            Event::Start(start) if in_body => {
                warnings.warn(
                    "docx_unsupported_body_content",
                    "Unsupported DOCX body content was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(&mut reader, &end, DOCUMENT_XML)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(parsed.into_iter().map(|item| item.block).collect())
}

fn parse_table(
    reader: &mut Reader<&[u8]>,
    rels: &RelationshipMap,
    numbering: &NumberingMap,
    warnings: &mut WarningSink,
) -> Result<Table, DocxError> {
    let mut rows = Vec::new();
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"tr" => {
                rows.push(parse_table_row(reader, rels, numbering, warnings)?);
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"tbl" => break,
            Event::Start(start) => {
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(Table { rows })
}

fn parse_table_row(
    reader: &mut Reader<&[u8]>,
    rels: &RelationshipMap,
    numbering: &NumberingMap,
    warnings: &mut WarningSink,
) -> Result<TableRow, DocxError> {
    let mut cells = Vec::new();
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"tc" => {
                cells.push(parse_table_cell(reader, rels, numbering, warnings)?);
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"tr" => break,
            Event::Start(start) => {
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(TableRow { cells })
}

fn parse_table_cell(
    reader: &mut Reader<&[u8]>,
    rels: &RelationshipMap,
    numbering: &NumberingMap,
    warnings: &mut WarningSink,
) -> Result<TableCell, DocxError> {
    let mut parsed = Vec::new();
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"p" => {
                let block = parse_paragraph(reader, rels, numbering, warnings)?;
                push_parsed_block(&mut parsed, block);
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"tbl" => {
                warnings.warn(
                    "docx_nested_table_degraded",
                    "Nested DOCX tables were imported as plain visible text",
                );
                let table = parse_table(reader, rels, numbering, warnings)?;
                push_parsed_block(
                    &mut parsed,
                    ParsedBlock {
                        block: table_to_paragraph_block(&table),
                        list_marker: None,
                    },
                );
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"tc" => break,
            Event::Empty(_) => {
                warnings.warn(
                    "docx_unsupported_table_content",
                    "Unsupported DOCX table content was ignored during import",
                );
            }
            Event::Start(start) => {
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }

    let blocks = parsed
        .into_iter()
        .map(|item| item.block)
        .collect::<Vec<_>>();
    Ok(TableCell {
        blocks: if blocks.is_empty() {
            vec![empty_paragraph_block()]
        } else {
            blocks
        },
    })
}

fn parse_paragraph(
    reader: &mut Reader<&[u8]>,
    rels: &RelationshipMap,
    numbering: &NumberingMap,
    warnings: &mut WarningSink,
) -> Result<ParsedBlock, DocxError> {
    let mut properties = ParagraphProperties::default();
    let mut inlines = Vec::new();

    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"pPr" => {
                properties = parse_paragraph_properties(reader, numbering, warnings)?;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"r" => {
                let run = parse_run(reader, None, warnings)?;
                append_inlines(&mut inlines, run);
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"hyperlink" => {
                let link = hyperlink_ref(&start, rels, warnings)?;
                let run = parse_hyperlink(reader, link, warnings)?;
                append_inlines(&mut inlines, run);
            }
            Event::Start(start)
                if matches!(
                    local_name(start.name().as_ref()),
                    b"drawing" | b"object" | b"pict"
                ) =>
            {
                warnings.warn(
                    "docx_media_ignored",
                    "Unsupported DOCX media content was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"p" => break,
            Event::Empty(_) => {
                warnings.warn(
                    "docx_unsupported_paragraph_content",
                    "Unsupported DOCX paragraph content was ignored during import",
                );
            }
            Event::Start(start) => {
                warnings.warn(
                    "docx_unsupported_paragraph_content",
                    "Unsupported DOCX paragraph content was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }

    let block = if let Some(level) = properties.heading_level {
        Block::Heading(Heading {
            bookmark_id: None,
            level,
            inlines,
        })
    } else {
        Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: Default::default(),
            inlines,
        })
    };

    Ok(ParsedBlock {
        block,
        list_marker: properties.list_marker,
    })
}

fn parse_paragraph_properties(
    reader: &mut Reader<&[u8]>,
    numbering: &NumberingMap,
    warnings: &mut WarningSink,
) -> Result<ParagraphProperties, DocxError> {
    let mut properties = ParagraphProperties::default();

    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"pStyle" =>
            {
                if let Some(style) = attr_value(&start, b"val", DOCUMENT_XML)? {
                    properties.heading_level = heading_level_from_style(&style);
                }
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"numPr" => {
                properties.list_marker = parse_num_properties(reader, numbering, warnings)?;
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"numPr" => {}
            Event::End(end) if local_name(end.name().as_ref()) == b"pPr" => break,
            Event::Start(start) => {
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(properties)
}

fn parse_num_properties(
    reader: &mut Reader<&[u8]>,
    numbering: &NumberingMap,
    warnings: &mut WarningSink,
) -> Result<Option<ListMarker>, DocxError> {
    let mut properties = NumberingProperties::default();
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"numId" =>
            {
                properties.num_id = attr_value(&start, b"val", DOCUMENT_XML)?;
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"ilvl" =>
            {
                properties.level = attr_value(&start, b"val", DOCUMENT_XML)?
                    .and_then(|value| value.parse::<u8>().ok())
                    .map(|value| value.saturating_add(1).clamp(1, 9));
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"numPr" => break,
            Event::Start(start) => {
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }

    let Some(num_id) = properties.num_id else {
        return Ok(None);
    };
    let ordered = numbering.ordered_for_num_id(&num_id).unwrap_or_else(|| {
        warnings.warn(
            "docx_list_markers_degraded",
            "DOCX lists were imported with generic list markers",
        );
        false
    });
    Ok(Some(ListMarker {
        ordered,
        level: properties.level.unwrap_or(1),
    }))
}

fn parse_hyperlink(
    reader: &mut Reader<&[u8]>,
    link: HyperlinkRef,
    warnings: &mut WarningSink,
) -> Result<Vec<Inline>, DocxError> {
    let mut inlines = Vec::new();
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"r" => {
                let run = parse_run(reader, link.href.clone(), warnings)?;
                append_inlines(&mut inlines, run);
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"hyperlink" => break,
            Event::Start(start) => {
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(inlines)
}

fn parse_run(
    reader: &mut Reader<&[u8]>,
    link: Option<String>,
    warnings: &mut WarningSink,
) -> Result<Vec<Inline>, DocxError> {
    let mut properties = RunProperties::default();
    let mut text = String::new();

    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"rPr" => {
                properties = parse_run_properties(reader)?;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"t" => {
                text.push_str(&read_text_element(reader, b"t", DOCUMENT_XML)?);
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"tab" => {
                text.push('\t');
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"br" => {
                text.push('\n');
            }
            Event::Empty(start)
                if matches!(
                    local_name(start.name().as_ref()),
                    b"drawing" | b"object" | b"pict"
                ) =>
            {
                warnings.warn(
                    "docx_media_ignored",
                    "Unsupported DOCX media content was ignored during import",
                );
            }
            Event::Start(start)
                if matches!(
                    local_name(start.name().as_ref()),
                    b"drawing" | b"object" | b"pict"
                ) =>
            {
                warnings.warn(
                    "docx_media_ignored",
                    "Unsupported DOCX media content was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Empty(start)
                if matches!(
                    local_name(start.name().as_ref()),
                    b"footnoteReference"
                        | b"endnoteReference"
                        | b"commentReference"
                        | b"fldChar"
                        | b"instrText"
                ) =>
            {
                warnings.warn(
                    "docx_inline_metadata_ignored",
                    "Unsupported DOCX inline metadata was ignored during import",
                );
            }
            Event::Start(start)
                if matches!(
                    local_name(start.name().as_ref()),
                    b"footnoteReference"
                        | b"endnoteReference"
                        | b"commentReference"
                        | b"fldChar"
                        | b"instrText"
                ) =>
            {
                warnings.warn(
                    "docx_inline_metadata_ignored",
                    "Unsupported DOCX inline metadata was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"r" => break,
            Event::Empty(_) => {
                warnings.warn(
                    "docx_unsupported_run_content",
                    "Unsupported DOCX run content was ignored during import",
                );
            }
            Event::Start(start) => {
                warnings.warn(
                    "docx_unsupported_run_content",
                    "Unsupported DOCX run content was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }

    if text.is_empty() {
        return Ok(Vec::new());
    }
    Ok(vec![Inline {
        text,
        marks: properties.marks(),
        link,
        comment_ids: Vec::new(),
        style: Default::default(),
        field: None,
        note_reference: None,
        tracked_change: None,
    }])
}

fn parse_run_properties(reader: &mut Reader<&[u8]>) -> Result<RunProperties, DocxError> {
    let mut properties = RunProperties::default();
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"b" =>
            {
                properties.bold = truthy_word_bool(&start, DOCUMENT_XML)?;
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"i" =>
            {
                properties.italic = truthy_word_bool(&start, DOCUMENT_XML)?;
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"u" =>
            {
                properties.underline =
                    attr_value(&start, b"val", DOCUMENT_XML)?.as_deref() != Some("none");
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"rPr" => break,
            Event::Start(start) => {
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(properties)
}

fn hyperlink_ref(
    start: &BytesStart<'_>,
    rels: &RelationshipMap,
    warnings: &mut WarningSink,
) -> Result<HyperlinkRef, DocxError> {
    if let Some(anchor) = attr_value(start, b"anchor", DOCUMENT_XML)? {
        let href = sanitize_bookmark_id(&anchor).map(|value| format!("#{value}"));
        if href.is_none() {
            warnings.warn(
                "docx_unsafe_hyperlink",
                "Unsafe DOCX hyperlinks were stripped during import",
            );
        }
        return Ok(HyperlinkRef { href });
    }

    let id = attr_value(start, b"id", DOCUMENT_XML)?;
    let href = id
        .as_deref()
        .and_then(|id| rels.hyperlinks.get(id))
        .cloned();
    if id.is_some() && href.is_none() {
        warnings.warn(
            "docx_missing_hyperlink_target",
            "DOCX hyperlinks with missing or unsupported targets were imported as plain text",
        );
    }
    Ok(HyperlinkRef { href })
}

fn push_parsed_block(blocks: &mut Vec<ParsedBlock>, parsed: ParsedBlock) {
    let Some(marker) = parsed.list_marker else {
        blocks.push(parsed);
        return;
    };
    let definition_id = if marker.ordered {
        "900w-ordered"
    } else {
        "900w-unordered"
    }
    .to_string();

    if let Some(ParsedBlock {
        block: Block::List(list),
        ..
    }) = blocks.last_mut()
    {
        if list.definition_id == definition_id {
            list.items.push(ListItem {
                level: marker.level,
                blocks: vec![parsed.block],
            });
            return;
        }
    }

    blocks.push(ParsedBlock {
        block: Block::List(ListBlock {
            definition_id,
            items: vec![ListItem {
                level: marker.level,
                blocks: vec![parsed.block],
            }],
        }),
        list_marker: None,
    });
}

fn append_inlines(target: &mut Vec<Inline>, source: Vec<Inline>) {
    for inline in source {
        if inline.text.is_empty() {
            continue;
        }
        if let Some(previous) = target.last_mut() {
            if previous.marks == inline.marks
                && previous.link == inline.link
                && previous.style == inline.style
                && previous.comment_ids.is_empty()
                && inline.comment_ids.is_empty()
                && previous.field.is_none()
                && inline.field.is_none()
                && previous.note_reference.is_none()
                && inline.note_reference.is_none()
                && previous.tracked_change.is_none()
                && inline.tracked_change.is_none()
            {
                previous.text.push_str(&inline.text);
                continue;
            }
        }
        target.push(inline);
    }
}

fn read_text_element(
    reader: &mut Reader<&[u8]>,
    end_name: &[u8],
    name: &str,
) -> Result<String, DocxError> {
    let mut value = String::new();
    loop {
        match reader.read_event().map_err(|err| xml_error(name, err))? {
            Event::Text(text) => {
                value.push_str(&text.xml10_content().map_err(|err| DocxError::Xml {
                    name: name.to_string(),
                    message: err.to_string(),
                })?)
            }
            Event::CData(text) => value.push_str(&String::from_utf8_lossy(&text.into_inner())),
            Event::End(end) if local_name(end.name().as_ref()) == end_name => break,
            Event::Start(start) => {
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, name)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(value)
}

fn skip_element(reader: &mut Reader<&[u8]>, _end_name: &[u8], name: &str) -> Result<(), DocxError> {
    let mut depth = 1_usize;
    loop {
        match reader.read_event().map_err(|err| xml_error(name, err))? {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(())
}

fn heading_level_from_style(style: &str) -> Option<u8> {
    let compact = style
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    match compact.as_str() {
        "heading1" | "h1" => Some(1),
        "heading2" | "h2" => Some(2),
        "heading3" | "h3" => Some(3),
        _ => None,
    }
}

fn truthy_word_bool(start: &BytesStart<'_>, name: &str) -> Result<bool, DocxError> {
    Ok(!matches!(
        attr_value(start, b"val", name)?.as_deref(),
        Some("0" | "false" | "off")
    ))
}

fn empty_paragraph_block() -> Block {
    Block::Paragraph(Paragraph {
        bookmark_id: None,
        style: StyleId::from("body"),
        format: Default::default(),
        inlines: Vec::new(),
    })
}

fn table_to_paragraph_block(table: &Table) -> Block {
    let mut text = String::new();
    for row in &table.rows {
        if !text.is_empty() {
            text.push('\n');
        }
        for (index, cell) in row.cells.iter().enumerate() {
            if index > 0 {
                text.push('\t');
            }
            text.push_str(&blocks_text(&cell.blocks));
        }
    }
    Block::Paragraph(Paragraph {
        bookmark_id: None,
        style: StyleId::from("body"),
        format: Default::default(),
        inlines: vec![Inline::text(text)],
    })
}

fn render_content_types_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/>
</Types>"#
        .to_string()
}

fn render_root_rels_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="{REL_TYPE_OFFICE_DOCUMENT}" Target="word/document.xml"/>
</Relationships>"#
    )
}

fn render_document_rels_xml(hyperlinks: &HyperlinkIds) -> String {
    let mut output = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="{REL_TYPE_STYLES}" Target="styles.xml"/>
  <Relationship Id="rId2" Type="{REL_TYPE_NUMBERING}" Target="numbering.xml"/>"#
    );
    for (href, id) in &hyperlinks.external {
        output.push_str("\n  <Relationship Id=\"");
        output.push_str(&escape_xml(id));
        output.push_str("\" Type=\"");
        output.push_str(REL_TYPE_HYPERLINK);
        output.push_str("\" Target=\"");
        output.push_str(&escape_xml(href));
        output.push_str("\" TargetMode=\"External\"/>");
    }
    output.push_str("\n</Relationships>");
    output
}

fn render_document_xml(document: &Document, hyperlinks: &HyperlinkIds) -> String {
    let mut output = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<w:body>"#,
    );

    for section in &document.sections {
        for block in &section.blocks {
            render_block_xml(block, hyperlinks, &mut output);
        }
    }

    output.push_str(
        r#"<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr>"#,
    );
    output.push_str("</w:body></w:document>");
    output
}

fn render_block_xml(block: &Block, hyperlinks: &HyperlinkIds, output: &mut String) {
    match block {
        Block::Paragraph(paragraph) => render_paragraph_xml(paragraph, None, hyperlinks, output),
        Block::Heading(heading) => render_heading_xml(heading, hyperlinks, output),
        Block::List(list) => render_list_xml(list, hyperlinks, output),
        Block::Table(table) => render_table_xml(table, hyperlinks, output),
        Block::TableOfContents(table_of_contents) => render_fallback_paragraph(
            &table_of_contents_text(table_of_contents),
            hyperlinks,
            output,
        ),
        Block::Image(image) => {
            let mut text = String::new();
            if let Some(alt) = image
                .alt_text
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                text.push_str(alt.trim());
            }
            if let Some(caption) = image
                .presentation
                .caption
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(caption.trim());
            }
            render_fallback_paragraph(&text, hyperlinks, output);
        }
        Block::PageBreak => {
            output.push_str("<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>");
        }
    }
}

fn render_heading_xml(heading: &Heading, hyperlinks: &HyperlinkIds, output: &mut String) {
    output.push_str("<w:p><w:pPr><w:pStyle w:val=\"Heading");
    output.push_str(&heading.level.clamp(1, 3).to_string());
    output.push_str("\"/></w:pPr>");
    render_inlines_xml(&heading.inlines, hyperlinks, output);
    output.push_str("</w:p>");
}

fn render_paragraph_xml(
    paragraph: &Paragraph,
    list_marker: Option<ListMarker>,
    hyperlinks: &HyperlinkIds,
    output: &mut String,
) {
    output.push_str("<w:p>");
    if let Some(marker) = list_marker {
        output.push_str("<w:pPr><w:numPr><w:ilvl w:val=\"");
        output.push_str(&marker.level.saturating_sub(1).to_string());
        output.push_str("\"/><w:numId w:val=\"");
        output.push_str(if marker.ordered { "2" } else { "1" });
        output.push_str("\"/></w:numPr></w:pPr>");
    }
    render_inlines_xml(&paragraph.inlines, hyperlinks, output);
    output.push_str("</w:p>");
}

fn render_list_xml(list: &ListBlock, hyperlinks: &HyperlinkIds, output: &mut String) {
    let ordered = list.definition_id == "900w-ordered";
    for item in &list.items {
        for block in &item.blocks {
            match block {
                Block::Paragraph(paragraph) => render_paragraph_xml(
                    paragraph,
                    Some(ListMarker {
                        ordered,
                        level: item.level.clamp(1, 9),
                    }),
                    hyperlinks,
                    output,
                ),
                Block::Heading(heading) => {
                    let paragraph = Paragraph {
                        bookmark_id: None,
                        style: StyleId::from("body"),
                        format: Default::default(),
                        inlines: heading.inlines.clone(),
                    };
                    render_paragraph_xml(
                        &paragraph,
                        Some(ListMarker {
                            ordered,
                            level: item.level.clamp(1, 9),
                        }),
                        hyperlinks,
                        output,
                    );
                }
                _ => render_fallback_paragraph(&block_text(block), hyperlinks, output),
            }
        }
    }
}

fn render_table_xml(table: &Table, hyperlinks: &HyperlinkIds, output: &mut String) {
    output.push_str("<w:tbl><w:tblPr><w:tblW w:w=\"0\" w:type=\"auto\"/></w:tblPr>");
    for row in &table.rows {
        output.push_str("<w:tr>");
        for cell in &row.cells {
            output.push_str("<w:tc><w:tcPr><w:tcW w:w=\"0\" w:type=\"auto\"/></w:tcPr>");
            if cell.blocks.is_empty() {
                output.push_str("<w:p/>");
            } else {
                for block in &cell.blocks {
                    match block {
                        Block::Paragraph(paragraph) => {
                            render_paragraph_xml(paragraph, None, hyperlinks, output)
                        }
                        Block::Heading(heading) => render_heading_xml(heading, hyperlinks, output),
                        Block::List(list) => render_list_xml(list, hyperlinks, output),
                        _ => render_fallback_paragraph(&block_text(block), hyperlinks, output),
                    }
                }
            }
            output.push_str("</w:tc>");
        }
        output.push_str("</w:tr>");
    }
    output.push_str("</w:tbl>");
}

fn render_fallback_paragraph(text: &str, hyperlinks: &HyperlinkIds, output: &mut String) {
    render_paragraph_xml(
        &Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: Default::default(),
            inlines: if text.is_empty() {
                Vec::new()
            } else {
                vec![Inline::text(text)]
            },
        },
        None,
        hyperlinks,
        output,
    );
}

fn render_inlines_xml(inlines: &[Inline], hyperlinks: &HyperlinkIds, output: &mut String) {
    if inlines.is_empty() {
        return;
    }
    for inline in inlines {
        if let Some(href) = inline.link.as_deref().and_then(sanitize_text_href) {
            if let Some(anchor) = href.strip_prefix('#') {
                output.push_str("<w:hyperlink w:anchor=\"");
                output.push_str(&escape_xml(anchor));
                output.push_str("\">");
                render_run_xml(inline, output);
                output.push_str("</w:hyperlink>");
                continue;
            }
            if let Some(id) = hyperlinks.external.get(&href) {
                output.push_str("<w:hyperlink r:id=\"");
                output.push_str(&escape_xml(id));
                output.push_str("\" w:history=\"1\">");
                render_run_xml(inline, output);
                output.push_str("</w:hyperlink>");
                continue;
            }
        }
        render_run_xml(inline, output);
    }
}

fn render_run_xml(inline: &Inline, output: &mut String) {
    if inline.text.is_empty() {
        return;
    }
    output.push_str("<w:r>");
    if !inline.marks.is_empty() {
        output.push_str("<w:rPr>");
        if inline.marks.contains(&InlineMark::Bold) {
            output.push_str("<w:b/>");
        }
        if inline.marks.contains(&InlineMark::Italic) {
            output.push_str("<w:i/>");
        }
        if inline.marks.contains(&InlineMark::Underline) {
            output.push_str("<w:u w:val=\"single\"/>");
        }
        output.push_str("</w:rPr>");
    }

    let mut text_buffer = String::new();
    for ch in inline.text.chars() {
        match ch {
            '\n' => {
                flush_text_run(&mut text_buffer, output);
                output.push_str("<w:br/>");
            }
            '\t' => {
                flush_text_run(&mut text_buffer, output);
                output.push_str("<w:tab/>");
            }
            _ => text_buffer.push(ch),
        }
    }
    flush_text_run(&mut text_buffer, output);
    output.push_str("</w:r>");
}

fn flush_text_run(text: &mut String, output: &mut String) {
    if text.is_empty() {
        return;
    }
    output.push_str("<w:t xml:space=\"preserve\">");
    output.push_str(&escape_xml(text));
    output.push_str("</w:t>");
    text.clear();
}

fn render_styles_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
  <w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/></w:pPr><w:rPr><w:b/><w:sz w:val="32"/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:qFormat/><w:rPr><w:b/><w:sz w:val="28"/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="Heading3"><w:name w:val="heading 3"/><w:basedOn w:val="Normal"/><w:qFormat/><w:rPr><w:b/><w:sz w:val="24"/></w:rPr></w:style>
</w:styles>"#
        .to_string()
}

fn render_numbering_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="1"><w:multiLevelType w:val="hybridMultilevel"/><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="bullet"/><w:lvlText w:val="•"/><w:lvlJc w:val="left"/></w:lvl></w:abstractNum>
  <w:abstractNum w:abstractNumId="2"><w:multiLevelType w:val="hybridMultilevel"/><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/><w:lvlJc w:val="left"/></w:lvl></w:abstractNum>
  <w:num w:numId="1"><w:abstractNumId w:val="1"/></w:num>
  <w:num w:numId="2"><w:abstractNumId w:val="2"/></w:num>
</w:numbering>"#
        .to_string()
}

fn collect_external_hyperlinks(document: &Document) -> BTreeSet<String> {
    let mut links = BTreeSet::new();
    for section in &document.sections {
        collect_external_hyperlinks_from_blocks(&section.blocks, &mut links);
    }
    links
}

fn collect_external_hyperlinks_from_blocks(blocks: &[Block], links: &mut BTreeSet<String>) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                collect_external_hyperlinks_from_inlines(&paragraph.inlines, links)
            }
            Block::Heading(heading) => {
                collect_external_hyperlinks_from_inlines(&heading.inlines, links)
            }
            Block::List(list) => {
                for item in &list.items {
                    collect_external_hyperlinks_from_blocks(&item.blocks, links);
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_external_hyperlinks_from_blocks(&cell.blocks, links);
                    }
                }
            }
            Block::TableOfContents(_) | Block::Image(_) | Block::PageBreak => {}
        }
    }
}

fn collect_external_hyperlinks_from_inlines(inlines: &[Inline], links: &mut BTreeSet<String>) {
    for inline in inlines {
        if let Some(href) = inline.link.as_deref().and_then(sanitize_text_href) {
            if !href.starts_with('#') {
                links.insert(href);
            }
        }
    }
}

fn assign_hyperlink_ids(links: &BTreeSet<String>) -> HyperlinkIds {
    let external = links
        .iter()
        .enumerate()
        .map(|(index, href)| (href.clone(), format!("rId{}", index + 3)))
        .collect();
    HyperlinkIds { external }
}

fn blocks_text(blocks: &[Block]) -> String {
    blocks.iter().map(block_text).collect::<Vec<_>>().join("\n")
}

fn block_text(block: &Block) -> String {
    match block {
        Block::Paragraph(paragraph) => inline_text(&paragraph.inlines),
        Block::Heading(heading) => inline_text(&heading.inlines),
        Block::TableOfContents(table_of_contents) => table_of_contents_text(table_of_contents),
        Block::List(list) => list
            .items
            .iter()
            .flat_map(|item| item.blocks.iter().map(block_text))
            .collect::<Vec<_>>()
            .join("\n"),
        Block::Table(table) => table
            .rows
            .iter()
            .map(|row| {
                row.cells
                    .iter()
                    .map(|cell| blocks_text(&cell.blocks))
                    .collect::<Vec<_>>()
                    .join("\t")
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Block::Image(image) => image
            .alt_text
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_string(),
        Block::PageBreak => String::new(),
    }
}

fn table_of_contents_text(table_of_contents: &word_core::TableOfContents) -> String {
    let mut lines = Vec::new();
    if !table_of_contents.title.trim().is_empty() {
        lines.push(table_of_contents.title.trim().to_string());
    }
    for entry in &table_of_contents.entries {
        lines.push(entry.text.clone());
    }
    lines.join("\n")
}

fn inline_text(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .map(|inline| inline.text.as_str())
        .collect::<String>()
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
    if lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("mailto:")
    {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn validate_entry_path(name: &str, limits: PackageLimits) -> Result<(), DocxError> {
    let normalized = name.trim_end_matches('/');
    if name.starts_with('/')
        || name.starts_with('\\')
        || name.contains('\\')
        || name.contains(':')
        || normalized.is_empty()
        || normalized
            .split('/')
            .any(|part| part == ".." || part.is_empty())
    {
        return Err(DocxError::UnsafePath {
            name: name.to_string(),
        });
    }
    if normalized.split('/').count() > limits.max_path_depth {
        return Err(DocxError::PathTooDeep {
            name: name.to_string(),
        });
    }
    Ok(())
}

fn validate_entry_mode(file: &zip::read::ZipFile<'_>, name: &str) -> Result<(), DocxError> {
    const UNIX_FILE_TYPE_MASK: u32 = 0o170000;
    const UNIX_SYMLINK: u32 = 0o120000;

    if let Some(mode) = file.unix_mode() {
        if mode & UNIX_FILE_TYPE_MASK == UNIX_SYMLINK {
            return Err(DocxError::SymlinkEntry {
                name: name.to_string(),
            });
        }
    }
    if file.encrypted() {
        return Err(DocxError::EncryptedEntry {
            name: name.to_string(),
        });
    }
    Ok(())
}

fn validate_entry_kind(name: &str) -> Result<(), DocxError> {
    let lower = name.to_ascii_lowercase();
    let executable = lower.contains("vbaproject.bin")
        || lower.starts_with("word/activex/")
        || lower.starts_with("word/embeddings/")
        || lower.starts_with("customxml/")
        || lower.starts_with("scripts/")
        || lower.ends_with(".exe")
        || lower.ends_with(".dll")
        || lower.ends_with(".dylib")
        || lower.ends_with(".so")
        || lower.ends_with(".js")
        || lower.ends_with(".sh")
        || lower.ends_with(".bat")
        || lower.ends_with(".cmd");

    if executable {
        return Err(DocxError::ExecutableEntry {
            name: name.to_string(),
        });
    }
    Ok(())
}

fn validate_xml_preflight(
    name: &str,
    content: &str,
    limits: PackageLimits,
) -> Result<(), DocxError> {
    let lower = content.to_ascii_lowercase();
    if lower.contains("<!doctype") || lower.contains("<!entity") {
        return Err(DocxError::XmlEntityDeclaration {
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
                    return Err(DocxError::XmlTooDeep {
                        name: name.to_string(),
                    });
                }
            }
            Event::End(_) => depth = depth.saturating_sub(1),
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(())
}

fn attr_value(
    start: &BytesStart<'_>,
    local: &[u8],
    name: &str,
) -> Result<Option<String>, DocxError> {
    for attr in start.attributes().with_checks(true) {
        let attr = attr.map_err(|err| DocxError::Xml {
            name: name.to_string(),
            message: err.to_string(),
        })?;
        if local_name(attr.key.as_ref()) == local {
            return Ok(Some(
                attr.decode_and_unescape_value(start.decoder())
                    .map_err(|err| xml_error(name, err))?
                    .into_owned(),
            ));
        }
    }
    Ok(None)
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn xml_error(name: &str, error: impl std::fmt::Display) -> DocxError {
    DocxError::Xml {
        name: name.to_string(),
        message: error.to_string(),
    }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use word_core::{InlineStyle, ParagraphFormat};

    #[test]
    fn imports_synthetic_docx_paragraphs_headings_marks_links_lists_and_tables() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<w:body>
  <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Project Plan</w:t></w:r></w:p>
  <w:p>
    <w:r><w:rPr><w:b/></w:rPr><w:t>Bold </w:t></w:r>
    <w:r><w:rPr><w:i/><w:u w:val="single"/></w:rPr><w:t>italic underline </w:t></w:r>
    <w:hyperlink r:id="rId9"><w:r><w:t>link</w:t></w:r></w:hyperlink>
  </w:p>
  <w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="7"/></w:numPr></w:pPr><w:r><w:t>First item</w:t></w:r></w:p>
  <w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="7"/></w:numPr></w:pPr><w:r><w:t>Second item</w:t></w:r></w:p>
  <w:tbl><w:tr><w:tc><w:p><w:r><w:t>A1</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>B1</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
</w:body></w:document>"#,
            Some(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.invalid/doc" TargetMode="External"/>
</Relationships>"#,
            ),
            Some(
                r#"<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:abstractNum w:abstractNumId="4"><w:lvl w:ilvl="0"><w:numFmt w:val="decimal"/></w:lvl></w:abstractNum>
<w:num w:numId="7"><w:abstractNumId w:val="4"/></w:num>
</w:numbering>"#,
            ),
        );

        let document = read_docx_bytes(&bytes).expect("docx should import");

        assert!(document.warnings.is_empty(), "{:?}", document.warnings);
        assert_eq!(document.sections[0].blocks.len(), 4);
        let Block::Heading(heading) = &document.sections[0].blocks[0] else {
            panic!("first block should import as heading");
        };
        assert_eq!(heading.level, 1);
        assert_eq!(heading.inlines[0].text, "Project Plan");

        let Block::Paragraph(paragraph) = &document.sections[0].blocks[1] else {
            panic!("second block should import as paragraph");
        };
        assert_eq!(paragraph.inlines[0].marks, vec![InlineMark::Bold]);
        assert_eq!(
            paragraph.inlines[1].marks,
            vec![InlineMark::Italic, InlineMark::Underline]
        );
        assert_eq!(
            paragraph.inlines[2].link.as_deref(),
            Some("https://example.invalid/doc")
        );

        let Block::List(list) = &document.sections[0].blocks[2] else {
            panic!("numbered paragraphs should group into a list");
        };
        assert_eq!(list.definition_id, "900w-ordered");
        assert_eq!(list.items.len(), 2);

        let Block::Table(table) = &document.sections[0].blocks[3] else {
            panic!("fourth block should import as table");
        };
        assert_eq!(table.rows[0].cells.len(), 2);
    }

    #[test]
    fn imports_unsafe_hyperlinks_as_plain_text_with_warning() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<w:body><w:p><w:hyperlink r:id="rId9"><w:r><w:t>bad link</w:t></w:r></w:hyperlink></w:p></w:body></w:document>"#,
            Some(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="file:///private/doc" TargetMode="External"/>
</Relationships>"#,
            ),
            None,
        );

        let document = read_docx_bytes(&bytes).expect("docx should import with warning");
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("paragraph expected");
        };
        assert_eq!(paragraph.inlines[0].text, "bad link");
        assert_eq!(paragraph.inlines[0].link, None);
        assert!(document
            .warnings
            .iter()
            .any(|warning| warning.code == "docx_unsafe_hyperlink"));
    }

    #[test]
    fn exports_minimal_docx_that_imports_supported_content() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::Heading(Heading {
                bookmark_id: None,
                level: 2,
                inlines: vec![Inline::text("Heading")],
            }),
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: StyleId::from("body"),
                format: ParagraphFormat::default(),
                inlines: vec![Inline {
                    text: "Linked bold".to_string(),
                    marks: vec![InlineMark::Bold],
                    link: Some("https://example.invalid/export".to_string()),
                    comment_ids: Vec::new(),
                    style: InlineStyle::default(),
                    field: None,
                    note_reference: None,
                    tracked_change: None,
                }],
            }),
            Block::List(ListBlock {
                definition_id: "900w-unordered".to_string(),
                items: vec![ListItem {
                    level: 1,
                    blocks: vec![Block::Paragraph(Paragraph {
                        bookmark_id: None,
                        style: StyleId::from("body"),
                        format: ParagraphFormat::default(),
                        inlines: vec![Inline::text("List item")],
                    })],
                }],
            }),
            Block::Table(Table {
                rows: vec![TableRow {
                    cells: vec![TableCell {
                        blocks: vec![Block::Paragraph(Paragraph {
                            bookmark_id: None,
                            style: StyleId::from("body"),
                            format: ParagraphFormat::default(),
                            inlines: vec![Inline::text("Cell")],
                        })],
                    }],
                }],
            }),
        ];

        let bytes = write_docx_bytes(&document).expect("docx should write");
        validate_docx_package(&bytes, PackageLimits::default()).expect("written package validates");
        let parsed = read_docx_bytes(&bytes).expect("written package should import");

        assert_eq!(parsed.sections[0].blocks.len(), 4);
        let Block::Heading(heading) = &parsed.sections[0].blocks[0] else {
            panic!("heading should round-trip through docx converter");
        };
        assert_eq!(heading.level, 2);
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[1] else {
            panic!("paragraph should round-trip through docx converter");
        };
        assert_eq!(paragraph.inlines[0].marks, vec![InlineMark::Bold]);
        assert_eq!(
            paragraph.inlines[0].link.as_deref(),
            Some("https://example.invalid/export")
        );
    }

    #[test]
    fn rejects_docx_with_unsafe_entry_without_path_leak_in_ui_layer() {
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer
            .start_file("../secret.xml", options)
            .expect("test zip should start");
        writer.write_all(b"<xml/>").expect("test zip should write");
        writer.finish().expect("test zip should finish");

        let err = validate_docx_package(&cursor.into_inner(), PackageLimits::default())
            .expect_err("unsafe path should fail");

        assert!(matches!(err, DocxError::UnsafePath { .. }));
    }

    #[test]
    fn rejects_docx_with_windows_drive_like_entry() {
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer
            .start_file(DOCUMENT_XML, options)
            .expect("document should start");
        writer
            .write_all(
                br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body/></w:document>"#,
            )
            .expect("document should write");
        writer
            .start_file("C:/placeholder-private-document/document.xml", options)
            .expect("drive-like path should start");
        writer
            .write_all(b"<xml/>")
            .expect("drive-like path should write");
        writer.finish().expect("test zip should finish");

        let err = validate_docx_package(&cursor.into_inner(), PackageLimits::default())
            .expect_err("drive-like path should fail");

        assert!(matches!(err, DocxError::UnsafePath { .. }));
    }

    #[test]
    fn rejects_docx_with_unsafe_directory_entry() {
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer
            .start_file(DOCUMENT_XML, options)
            .expect("document should start");
        writer
            .write_all(
                br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body/></w:document>"#,
            )
            .expect("document should write");
        writer
            .add_directory("../placeholder/", options)
            .expect("unsafe directory should start");
        writer.finish().expect("test zip should finish");

        let err = validate_docx_package(&cursor.into_inner(), PackageLimits::default())
            .expect_err("unsafe directory path should fail");

        assert!(matches!(err, DocxError::UnsafePath { .. }));
    }

    #[test]
    fn rejects_docx_with_disallowed_directory_entry() {
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer
            .start_file(DOCUMENT_XML, options)
            .expect("document should start");
        writer
            .write_all(
                br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body/></w:document>"#,
            )
            .expect("document should write");
        writer
            .add_directory("customXml/", options)
            .expect("disallowed directory should start");
        writer.finish().expect("test zip should finish");

        let err = validate_docx_package(&cursor.into_inner(), PackageLimits::default())
            .expect_err("disallowed directory path should fail");

        assert!(matches!(err, DocxError::ExecutableEntry { .. }));
    }

    #[test]
    fn rejects_docx_with_entity_declaration() {
        let bytes = synthetic_docx(
            r#"<!DOCTYPE w:document [<!ENTITY leak SYSTEM "file:///private/path">]>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body/></w:document>"#,
            None,
            None,
        );

        let err = validate_docx_package(&bytes, PackageLimits::default())
            .expect_err("entity declarations should fail preflight");

        assert!(matches!(err, DocxError::XmlEntityDeclaration { .. }));
    }

    #[test]
    fn rejects_docx_with_macro_or_custom_xml_entries() {
        for unsafe_entry in ["word/vbaProject.bin", "customXml/item1.xml"] {
            let mut cursor = Cursor::new(Vec::new());
            let mut writer = ZipWriter::new(&mut cursor);
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            writer
                .start_file(DOCUMENT_XML, options)
                .expect("document should start");
            writer
                .write_all(
                    br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body/></w:document>"#,
                )
                .expect("document should write");
            writer
                .start_file(unsafe_entry, options)
                .expect("unsafe entry should start");
            writer
                .write_all(b"payload")
                .expect("unsafe entry should write");
            writer.finish().expect("test zip should finish");

            let err = validate_docx_package(&cursor.into_inner(), PackageLimits::default())
                .expect_err("unsafe entry should fail");

            assert!(matches!(err, DocxError::ExecutableEntry { .. }));
        }
    }

    fn synthetic_docx(
        document_xml: &str,
        rels_xml: Option<&str>,
        numbering_xml: Option<&str>,
    ) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer
            .start_file("[Content_Types].xml", options)
            .expect("content types should start");
        writer
            .write_all(render_content_types_xml().as_bytes())
            .expect("content types should write");
        writer
            .start_file("_rels/.rels", options)
            .expect("root rels should start");
        writer
            .write_all(render_root_rels_xml().as_bytes())
            .expect("root rels should write");
        writer
            .start_file(DOCUMENT_XML, options)
            .expect("document should start");
        writer
            .write_all(document_xml.as_bytes())
            .expect("document should write");
        if let Some(rels_xml) = rels_xml {
            writer
                .start_file(DOCUMENT_RELS, options)
                .expect("rels should start");
            writer
                .write_all(rels_xml.as_bytes())
                .expect("rels should write");
        }
        if let Some(numbering_xml) = numbering_xml {
            writer
                .start_file(NUMBERING_XML, options)
                .expect("numbering should start");
            writer
                .write_all(numbering_xml.as_bytes())
                .expect("numbering should write");
        }
        writer.finish().expect("zip should finish");
        cursor.into_inner()
    }
}
