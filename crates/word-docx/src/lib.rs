use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};
use thiserror::Error;
use word_core::{
    sanitize_bookmark_id, AssetRef, Block, Document, DocumentWarning, Heading, ImageBlock,
    ImagePresentation, Inline, InlineMark, ListBlock, ListItem, PageField, PageRegion,
    PageRegionBlock, PageRegionParagraph, PageRegions, Paragraph, StyleId, Table, TableCell,
    TableRow,
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
const REL_TYPE_HEADER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/header";
const REL_TYPE_FOOTER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer";
const REL_TYPE_IMAGE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
const PAGE_REGION_XML: &str = "DOCX page region";
const MAX_DOCX_IMAGE_PARTS: usize = 64;
const MAX_DOCX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;

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
    page_regions: BTreeMap<String, PageRegionRelationship>,
    images: BTreeMap<String, DocxImageRelationship>,
}

#[derive(Debug, Clone)]
struct PageRegionRelationship {
    kind: PageRegionPartKind,
    target: String,
}

#[derive(Debug, Clone)]
struct DocxImageRelationship {
    target: String,
    expected_media_type: &'static str,
}

#[derive(Debug, Clone)]
struct ImportedDocxImage {
    asset_id: String,
}

#[derive(Debug, Clone, Default)]
struct ImportedDocxImages {
    by_relationship_id: BTreeMap<String, ImportedDocxImage>,
    assets: BTreeMap<String, AssetRef>,
}

#[derive(Debug, Clone, Default)]
struct DocxImageExports {
    parts: Vec<DocxImageExport>,
}

#[derive(Debug, Clone)]
struct DocxImageExport {
    asset_id: String,
    rel_id: String,
    path: String,
    target: String,
    media_type: &'static str,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PageRegionPartKind {
    Header,
    Footer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageRegionReferenceKind {
    DefaultHeader,
    DefaultFooter,
    FirstHeader,
    FirstFooter,
}

#[derive(Debug, Clone, Default)]
struct ParsedDocument {
    blocks: Vec<Block>,
    page_regions: PageRegions,
}

#[derive(Debug, Clone, Default)]
struct PageRegionReferences {
    header: Option<String>,
    footer: Option<String>,
    first_header: Option<String>,
    first_footer: Option<String>,
    different_first_page: bool,
}

#[derive(Debug, Clone, Default)]
struct DocxPageRegionExports {
    parts: Vec<DocxPageRegionPart>,
}

#[derive(Debug, Clone)]
struct DocxPageRegionPart {
    reference: PageRegionReferenceKind,
    kind: PageRegionPartKind,
    rel_id: String,
    path: &'static str,
    target: &'static str,
    blocks: Vec<PageRegionBlock>,
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

#[derive(Debug, Clone, Default)]
struct HyperlinkIds {
    external: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
enum ParagraphContent {
    Inline(Box<Inline>),
    Image(ImageBlock),
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
    let imported_images = read_docx_image_parts(&mut archive, &rels, &mut warnings)?;
    let page_region_part_xml = read_page_region_parts(&mut archive, &rels, &mut warnings)?;
    let numbering = if numbering_xml.is_empty() {
        NumberingMap::default()
    } else {
        parse_numbering_xml(&numbering_xml, &mut warnings)?
    };
    let parsed_document = parse_document_xml(
        &document_xml,
        &rels,
        &imported_images,
        &page_region_part_xml,
        &numbering,
        &mut warnings,
    )?;

    let mut document = Document::new_untitled();
    if let Some(section) = document.sections.first_mut() {
        section.blocks = if parsed_document.blocks.is_empty() {
            vec![empty_paragraph_block()]
        } else {
            parsed_document.blocks
        };
        section.page_regions = parsed_document.page_regions;
    }
    document.warnings = warnings.warnings;
    let mut referenced_assets = BTreeSet::new();
    let mut ordered_assets = Vec::new();
    for section in &document.sections {
        collect_image_asset_ids_from_blocks(
            &section.blocks,
            &mut referenced_assets,
            &mut ordered_assets,
        );
    }
    document.assets = imported_images
        .assets
        .into_iter()
        .filter(|(asset_id, _)| referenced_assets.contains(asset_id))
        .collect();
    Ok(document)
}

pub fn write_docx_bytes(document: &Document) -> Result<Vec<u8>, DocxError> {
    let hyperlinks = collect_external_hyperlinks(document);
    let hyperlink_ids = assign_hyperlink_ids(&hyperlinks);
    let (image_exports, next_rel_id) = collect_docx_image_exports(document, hyperlinks.len() + 3);
    let page_region_exports = collect_page_region_exports(document, next_rel_id);
    let compressed = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);

    writer.start_file("[Content_Types].xml", compressed)?;
    writer.write_all(render_content_types_xml(&page_region_exports, &image_exports).as_bytes())?;

    writer.start_file("_rels/.rels", compressed)?;
    writer.write_all(render_root_rels_xml().as_bytes())?;

    writer.start_file(DOCUMENT_XML, compressed)?;
    writer.write_all(
        render_document_xml(
            document,
            &hyperlink_ids,
            &page_region_exports,
            &image_exports,
        )
        .as_bytes(),
    )?;

    writer.start_file(DOCUMENT_RELS, compressed)?;
    writer.write_all(
        render_document_rels_xml(&hyperlink_ids, &page_region_exports, &image_exports).as_bytes(),
    )?;

    writer.start_file("word/styles.xml", compressed)?;
    writer.write_all(render_styles_xml().as_bytes())?;

    writer.start_file(NUMBERING_XML, compressed)?;
    writer.write_all(render_numbering_xml().as_bytes())?;

    for part in &page_region_exports.parts {
        writer.start_file(part.path, compressed)?;
        writer.write_all(render_page_region_part_xml(part).as_bytes())?;
    }
    for part in &image_exports.parts {
        writer.start_file(&part.path, compressed)?;
        writer.write_all(&part.bytes)?;
    }

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
                        if !target_mode_is_external(target_mode.as_deref()) {
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
                    (Some(id), Some(rel_type), Some(target))
                        if rel_type == REL_TYPE_HEADER || rel_type == REL_TYPE_FOOTER =>
                    {
                        if target_mode_is_external(target_mode.as_deref()) {
                            warnings.warn(
                                "docx_page_region_relationship_ignored",
                                "Unsupported DOCX header or footer relationships were ignored during import",
                            );
                            continue;
                        }
                        let kind = if rel_type == REL_TYPE_HEADER {
                            PageRegionPartKind::Header
                        } else {
                            PageRegionPartKind::Footer
                        };
                        if let Some(target) = resolve_page_region_target(&target, kind) {
                            relationships
                                .page_regions
                                .insert(id, PageRegionRelationship { kind, target });
                        } else {
                            warnings.warn(
                                "docx_page_region_relationship_ignored",
                                "Unsupported DOCX header or footer relationships were ignored during import",
                            );
                        }
                    }
                    (Some(id), Some(rel_type), Some(target)) if rel_type == REL_TYPE_IMAGE => {
                        if target_mode_is_external(target_mode.as_deref()) {
                            warnings.warn(
                                "docx_image_relationship_ignored",
                                "Unsupported DOCX image relationships were ignored during import",
                            );
                            continue;
                        }
                        if relationships.images.len() >= MAX_DOCX_IMAGE_PARTS {
                            warnings.warn(
                                "docx_too_many_images",
                                "Excess DOCX images were ignored during import",
                            );
                            continue;
                        }
                        if let Some((target, expected_media_type)) = resolve_image_target(&target) {
                            relationships.images.insert(
                                id,
                                DocxImageRelationship {
                                    target,
                                    expected_media_type,
                                },
                            );
                        } else {
                            warnings.warn(
                                "docx_image_relationship_ignored",
                                "Unsupported DOCX image relationships were ignored during import",
                            );
                        }
                    }
                    (_, Some(_), _) if target_mode_is_external(target_mode.as_deref()) => {
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

fn read_page_region_parts<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    rels: &RelationshipMap,
    warnings: &mut WarningSink,
) -> Result<BTreeMap<String, String>, DocxError> {
    let mut parts = BTreeMap::new();
    for relationship in rels.page_regions.values() {
        if parts.contains_key(&relationship.target) {
            continue;
        }
        match archive.by_name(&relationship.target) {
            Ok(mut file) => {
                let mut xml = String::new();
                file.read_to_string(&mut xml)?;
                parts.insert(relationship.target.clone(), xml);
            }
            Err(zip::result::ZipError::FileNotFound) => {
                warnings.warn(
                    "docx_page_region_part_missing",
                    "DOCX headers or footers with missing content were ignored during import",
                );
            }
            Err(err) => return Err(err.into()),
        }
    }
    Ok(parts)
}

fn read_docx_image_parts<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    rels: &RelationshipMap,
    warnings: &mut WarningSink,
) -> Result<ImportedDocxImages, DocxError> {
    let mut imported = ImportedDocxImages::default();
    let mut total_bytes = 0_u64;
    for (relationship_id, relationship) in &rels.images {
        match archive.by_name(&relationship.target) {
            Ok(mut file) => {
                let size = file.size();
                if size == 0 || total_bytes.saturating_add(size) > MAX_DOCX_IMAGE_BYTES {
                    warnings.warn(
                        "docx_image_part_ignored",
                        "Unsupported DOCX image payloads were ignored during import",
                    );
                    continue;
                }
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)?;
                let Some(media_type) = detect_image_media_type(&bytes) else {
                    warnings.warn(
                        "docx_image_part_ignored",
                        "Unsupported DOCX image payloads were ignored during import",
                    );
                    continue;
                };
                if media_type != relationship.expected_media_type {
                    warnings.warn(
                        "docx_image_part_ignored",
                        "Unsupported DOCX image payloads were ignored during import",
                    );
                    continue;
                }
                total_bytes = total_bytes.saturating_add(bytes.len() as u64);
                let asset_id = generic_docx_image_id(imported.assets.len() + 1, media_type);
                imported.assets.insert(
                    asset_id.clone(),
                    AssetRef {
                        id: asset_id.clone(),
                        media_type: media_type.to_string(),
                        byte_len: bytes.len(),
                        bytes,
                        original_name: None,
                    },
                );
                imported
                    .by_relationship_id
                    .insert(relationship_id.clone(), ImportedDocxImage { asset_id });
            }
            Err(zip::result::ZipError::FileNotFound) => {
                warnings.warn(
                    "docx_image_part_missing",
                    "DOCX images with missing content were ignored during import",
                );
            }
            Err(err) => return Err(err.into()),
        }
    }
    Ok(imported)
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
    imported_images: &ImportedDocxImages,
    page_region_part_xml: &BTreeMap<String, String>,
    numbering: &NumberingMap,
    warnings: &mut WarningSink,
) -> Result<ParsedDocument, DocxError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut parsed = Vec::new();
    let mut page_regions = PageRegions::default();
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
                let blocks =
                    parse_paragraph(&mut reader, rels, imported_images, numbering, warnings)?;
                for block in blocks {
                    push_parsed_block(&mut parsed, block);
                }
            }
            Event::Start(start) if in_body && local_name(start.name().as_ref()) == b"tbl" => {
                let table = parse_table(&mut reader, rels, imported_images, numbering, warnings)?;
                push_parsed_block(
                    &mut parsed,
                    ParsedBlock {
                        block: Block::Table(table),
                        list_marker: None,
                    },
                );
            }
            Event::Start(start) if in_body && local_name(start.name().as_ref()) == b"sectPr" => {
                let references = parse_section_properties(&mut reader, warnings)?;
                page_regions =
                    build_page_regions(&references, rels, page_region_part_xml, warnings)?;
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

    Ok(ParsedDocument {
        blocks: parsed.into_iter().map(|item| item.block).collect(),
        page_regions,
    })
}

fn parse_section_properties(
    reader: &mut Reader<&[u8]>,
    warnings: &mut WarningSink,
) -> Result<PageRegionReferences, DocxError> {
    let mut references = PageRegionReferences::default();
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"headerReference" =>
            {
                apply_page_region_reference(
                    &mut references,
                    PageRegionPartKind::Header,
                    &start,
                    warnings,
                )?;
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"footerReference" =>
            {
                apply_page_region_reference(
                    &mut references,
                    PageRegionPartKind::Footer,
                    &start,
                    warnings,
                )?;
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"titlePg" =>
            {
                references.different_first_page = truthy_word_bool(&start, DOCUMENT_XML)?;
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"sectPr" => break,
            Event::Start(start) => {
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(references)
}

fn apply_page_region_reference(
    references: &mut PageRegionReferences,
    kind: PageRegionPartKind,
    start: &BytesStart<'_>,
    warnings: &mut WarningSink,
) -> Result<(), DocxError> {
    let Some(id) = attr_value(start, b"id", DOCUMENT_XML)? else {
        warnings.warn(
            "docx_page_region_reference_ignored",
            "Unsupported DOCX header or footer references were ignored during import",
        );
        return Ok(());
    };
    match (kind, attr_value(start, b"type", DOCUMENT_XML)?.as_deref()) {
        (PageRegionPartKind::Header, Some("first")) => {
            references.first_header = Some(id);
            references.different_first_page = true;
        }
        (PageRegionPartKind::Footer, Some("first")) => {
            references.first_footer = Some(id);
            references.different_first_page = true;
        }
        (PageRegionPartKind::Header, Some("even")) | (PageRegionPartKind::Footer, Some("even")) => {
            warnings.warn(
                "docx_even_page_regions_ignored",
                "DOCX even-page headers or footers are not imported as editable page regions yet",
            );
        }
        (PageRegionPartKind::Header, _) => references.header = Some(id),
        (PageRegionPartKind::Footer, _) => references.footer = Some(id),
    }
    Ok(())
}

fn build_page_regions(
    references: &PageRegionReferences,
    rels: &RelationshipMap,
    page_region_part_xml: &BTreeMap<String, String>,
    warnings: &mut WarningSink,
) -> Result<PageRegions, DocxError> {
    let mut page_regions = PageRegions {
        different_first_page: references.different_first_page,
        ..PageRegions::default()
    };
    if let Some(region) = parse_referenced_page_region(
        references.header.as_deref(),
        PageRegionPartKind::Header,
        rels,
        page_region_part_xml,
        warnings,
    )? {
        page_regions.header = region;
    }
    if let Some(region) = parse_referenced_page_region(
        references.footer.as_deref(),
        PageRegionPartKind::Footer,
        rels,
        page_region_part_xml,
        warnings,
    )? {
        page_regions.footer = region;
    }
    if let Some(region) = parse_referenced_page_region(
        references.first_header.as_deref(),
        PageRegionPartKind::Header,
        rels,
        page_region_part_xml,
        warnings,
    )? {
        page_regions.first_header = region;
        page_regions.different_first_page = true;
    }
    if let Some(region) = parse_referenced_page_region(
        references.first_footer.as_deref(),
        PageRegionPartKind::Footer,
        rels,
        page_region_part_xml,
        warnings,
    )? {
        page_regions.first_footer = region;
        page_regions.different_first_page = true;
    }
    Ok(page_regions)
}

fn parse_referenced_page_region(
    id: Option<&str>,
    expected_kind: PageRegionPartKind,
    rels: &RelationshipMap,
    page_region_part_xml: &BTreeMap<String, String>,
    warnings: &mut WarningSink,
) -> Result<Option<PageRegion>, DocxError> {
    let Some(id) = id else {
        return Ok(None);
    };
    let Some(relationship) = rels.page_regions.get(id) else {
        warnings.warn(
            "docx_page_region_reference_ignored",
            "Unsupported DOCX header or footer references were ignored during import",
        );
        return Ok(None);
    };
    if relationship.kind != expected_kind {
        warnings.warn(
            "docx_page_region_reference_ignored",
            "Unsupported DOCX header or footer references were ignored during import",
        );
        return Ok(None);
    }
    let Some(xml) = page_region_part_xml.get(&relationship.target) else {
        return Ok(None);
    };
    Ok(Some(parse_page_region_xml(xml, warnings)?))
}

fn parse_table(
    reader: &mut Reader<&[u8]>,
    rels: &RelationshipMap,
    imported_images: &ImportedDocxImages,
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
                rows.push(parse_table_row(
                    reader,
                    rels,
                    imported_images,
                    numbering,
                    warnings,
                )?);
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
    imported_images: &ImportedDocxImages,
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
                cells.push(parse_table_cell(
                    reader,
                    rels,
                    imported_images,
                    numbering,
                    warnings,
                )?);
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
    imported_images: &ImportedDocxImages,
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
                let blocks = parse_paragraph(reader, rels, imported_images, numbering, warnings)?;
                for block in blocks {
                    push_parsed_block(&mut parsed, block);
                }
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"tbl" => {
                warnings.warn(
                    "docx_nested_table_degraded",
                    "Nested DOCX tables were imported as plain visible text",
                );
                let table = parse_table(reader, rels, imported_images, numbering, warnings)?;
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

fn parse_page_region_xml(xml: &str, warnings: &mut WarningSink) -> Result<PageRegion, DocxError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut blocks = Vec::new();

    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(PAGE_REGION_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"p" => {
                blocks.push(PageRegionBlock::Paragraph(parse_page_region_paragraph(
                    &mut reader,
                    warnings,
                )?));
            }
            Event::Start(start) if matches!(local_name(start.name().as_ref()), b"hdr" | b"ftr") => {
            }
            Event::End(end) if matches!(local_name(end.name().as_ref()), b"hdr" | b"ftr") => break,
            Event::Start(start) => {
                warnings.warn(
                    "docx_page_region_content_degraded",
                    "Unsupported DOCX header or footer content was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(&mut reader, &end, PAGE_REGION_XML)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(PageRegion {
        blocks,
        read_only: false,
    })
}

fn parse_page_region_paragraph(
    reader: &mut Reader<&[u8]>,
    warnings: &mut WarningSink,
) -> Result<PageRegionParagraph, DocxError> {
    let mut inlines = Vec::new();
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(PAGE_REGION_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"pPr" => {
                skip_element(reader, b"pPr", PAGE_REGION_XML)?;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"r" => {
                let run = parse_run(reader, None, warnings, PAGE_REGION_XML)?;
                append_inlines(&mut inlines, run);
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"fldSimple" => {
                let field = page_field_from_instruction(
                    attr_value(&start, b"instr", PAGE_REGION_XML)?.as_deref(),
                );
                let run = parse_simple_field(reader, field, warnings, PAGE_REGION_XML)?;
                append_inlines(&mut inlines, run);
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"fldSimple" => {
                if let Some(field) = page_field_from_instruction(
                    attr_value(&start, b"instr", PAGE_REGION_XML)?.as_deref(),
                ) {
                    inlines.push(Inline::field(field));
                } else {
                    warnings.warn(
                        "docx_field_degraded",
                        "Unsupported DOCX fields were imported as visible text when available",
                    );
                }
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"p" => break,
            Event::Start(start) => {
                warnings.warn(
                    "docx_page_region_content_degraded",
                    "Unsupported DOCX header or footer content was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, PAGE_REGION_XML)?;
            }
            Event::Empty(_) => {
                warnings.warn(
                    "docx_page_region_content_degraded",
                    "Unsupported DOCX header or footer content was ignored during import",
                );
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(PageRegionParagraph { inlines })
}

fn parse_paragraph(
    reader: &mut Reader<&[u8]>,
    rels: &RelationshipMap,
    imported_images: &ImportedDocxImages,
    numbering: &NumberingMap,
    warnings: &mut WarningSink,
) -> Result<Vec<ParsedBlock>, DocxError> {
    let mut properties = ParagraphProperties::default();
    let mut content = Vec::new();

    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"pPr" => {
                properties = parse_paragraph_properties(reader, numbering, warnings)?;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"r" => {
                let run = parse_paragraph_run(
                    reader,
                    None,
                    rels,
                    imported_images,
                    warnings,
                    DOCUMENT_XML,
                )?;
                append_paragraph_content(&mut content, run);
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"hyperlink" => {
                let link = hyperlink_ref(&start, rels, warnings)?;
                let run =
                    parse_hyperlink(reader, link, rels, imported_images, warnings, DOCUMENT_XML)?;
                append_paragraph_content(&mut content, run);
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"fldSimple" => {
                let field = page_field_from_instruction(
                    attr_value(&start, b"instr", DOCUMENT_XML)?.as_deref(),
                );
                let run = parse_simple_field(reader, field, warnings, DOCUMENT_XML)?;
                append_inline_content(&mut content, run);
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"fldSimple" => {
                if let Some(field) = page_field_from_instruction(
                    attr_value(&start, b"instr", DOCUMENT_XML)?.as_deref(),
                ) {
                    content.push(ParagraphContent::Inline(Box::new(Inline::field(field))));
                } else {
                    warnings.warn(
                        "docx_field_degraded",
                        "Unsupported DOCX fields were imported as visible text when available",
                    );
                }
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

    Ok(paragraph_content_to_blocks(content, &properties))
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
    rels: &RelationshipMap,
    imported_images: &ImportedDocxImages,
    warnings: &mut WarningSink,
    name: &str,
) -> Result<Vec<ParagraphContent>, DocxError> {
    let mut content = Vec::new();
    loop {
        match reader.read_event().map_err(|err| xml_error(name, err))? {
            Event::Start(start) if local_name(start.name().as_ref()) == b"r" => {
                let run = parse_paragraph_run(
                    reader,
                    link.href.clone(),
                    rels,
                    imported_images,
                    warnings,
                    name,
                )?;
                append_paragraph_content(&mut content, run);
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"hyperlink" => break,
            Event::Start(start) => {
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, name)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(content)
}

fn parse_paragraph_run(
    reader: &mut Reader<&[u8]>,
    link: Option<String>,
    rels: &RelationshipMap,
    imported_images: &ImportedDocxImages,
    warnings: &mut WarningSink,
    name: &str,
) -> Result<Vec<ParagraphContent>, DocxError> {
    let mut properties = RunProperties::default();
    let mut text = String::new();
    let mut content = Vec::new();

    loop {
        match reader.read_event().map_err(|err| xml_error(name, err))? {
            Event::Start(start) if local_name(start.name().as_ref()) == b"rPr" => {
                properties = parse_run_properties(reader, name)?;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"t" => {
                text.push_str(&read_text_element(reader, b"t", name)?);
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"tab" => {
                text.push('\t');
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"br" => {
                text.push('\n');
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"drawing" => {
                flush_run_text_content(&mut content, &mut text, &properties, link.clone());
                if let Some(image) = parse_drawing(reader, rels, imported_images, warnings, name)? {
                    content.push(ParagraphContent::Image(image));
                }
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"drawing" => {
                warnings.warn(
                    "docx_image_reference_ignored",
                    "Unsupported DOCX image references were ignored during import",
                );
            }
            Event::Empty(start)
                if matches!(local_name(start.name().as_ref()), b"object" | b"pict") =>
            {
                warnings.warn(
                    "docx_media_ignored",
                    "Unsupported DOCX media content was ignored during import",
                );
            }
            Event::Start(start)
                if matches!(local_name(start.name().as_ref()), b"object" | b"pict") =>
            {
                warnings.warn(
                    "docx_media_ignored",
                    "Unsupported DOCX media content was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, name)?;
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
                skip_element(reader, &end, name)?;
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
                skip_element(reader, &end, name)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }

    flush_run_text_content(&mut content, &mut text, &properties, link);
    Ok(content)
}

fn parse_drawing(
    reader: &mut Reader<&[u8]>,
    rels: &RelationshipMap,
    imported_images: &ImportedDocxImages,
    warnings: &mut WarningSink,
    name: &str,
) -> Result<Option<ImageBlock>, DocxError> {
    let mut embed_id = None;
    let mut linked_id = None;
    let mut alt_text = None;
    loop {
        match reader.read_event().map_err(|err| xml_error(name, err))? {
            Event::Empty(start) | Event::Start(start)
                if matches!(local_name(start.name().as_ref()), b"docPr" | b"cNvPr") =>
            {
                if alt_text.is_none() {
                    alt_text = image_alt_text(&start, name)?;
                }
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"blip" =>
            {
                if embed_id.is_none() {
                    embed_id = attr_value(&start, b"embed", name)?;
                }
                if linked_id.is_none() {
                    linked_id = attr_value(&start, b"link", name)?;
                }
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"drawing" => break,
            Event::Start(_) => {}
            Event::Eof => break,
            _ => {}
        }
    }

    if linked_id.is_some() {
        warnings.warn(
            "docx_image_reference_ignored",
            "Unsupported DOCX image references were ignored during import",
        );
    }
    let Some(embed_id) = embed_id else {
        warnings.warn(
            "docx_image_reference_ignored",
            "Unsupported DOCX image references were ignored during import",
        );
        return Ok(None);
    };
    if !rels.images.contains_key(&embed_id) {
        warnings.warn(
            "docx_image_reference_ignored",
            "Unsupported DOCX image references were ignored during import",
        );
        return Ok(None);
    }
    let Some(imported) = imported_images.by_relationship_id.get(&embed_id) else {
        warnings.warn(
            "docx_image_reference_ignored",
            "Unsupported DOCX image references were ignored during import",
        );
        return Ok(None);
    };
    Ok(Some(ImageBlock {
        asset_id: imported.asset_id.clone(),
        presentation: ImagePresentation::default(),
        alt_text,
    }))
}

fn image_alt_text(start: &BytesStart<'_>, name: &str) -> Result<Option<String>, DocxError> {
    let value = attr_value(start, b"descr", name)?.or(attr_value(start, b"title", name)?);
    Ok(value
        .map(|value| value.trim().chars().take(512).collect::<String>())
        .filter(|value| !value.is_empty()))
}

fn flush_run_text_content(
    content: &mut Vec<ParagraphContent>,
    text: &mut String,
    properties: &RunProperties,
    link: Option<String>,
) {
    if text.is_empty() {
        return;
    }
    content.push(ParagraphContent::Inline(Box::new(Inline {
        text: std::mem::take(text),
        marks: properties.marks(),
        link,
        comment_ids: Vec::new(),
        style: Default::default(),
        field: None,
        note_reference: None,
        tracked_change: None,
    })));
}

fn parse_run(
    reader: &mut Reader<&[u8]>,
    link: Option<String>,
    warnings: &mut WarningSink,
    name: &str,
) -> Result<Vec<Inline>, DocxError> {
    let mut properties = RunProperties::default();
    let mut text = String::new();

    loop {
        match reader.read_event().map_err(|err| xml_error(name, err))? {
            Event::Start(start) if local_name(start.name().as_ref()) == b"rPr" => {
                properties = parse_run_properties(reader, name)?;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"t" => {
                text.push_str(&read_text_element(reader, b"t", name)?);
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
                skip_element(reader, &end, name)?;
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
                skip_element(reader, &end, name)?;
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
                skip_element(reader, &end, name)?;
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

fn parse_run_properties(
    reader: &mut Reader<&[u8]>,
    name: &str,
) -> Result<RunProperties, DocxError> {
    let mut properties = RunProperties::default();
    loop {
        match reader.read_event().map_err(|err| xml_error(name, err))? {
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"b" =>
            {
                properties.bold = truthy_word_bool(&start, name)?;
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"i" =>
            {
                properties.italic = truthy_word_bool(&start, name)?;
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"u" =>
            {
                properties.underline = attr_value(&start, b"val", name)?.as_deref() != Some("none");
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"rPr" => break,
            Event::Start(start) => {
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, name)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(properties)
}

fn parse_simple_field(
    reader: &mut Reader<&[u8]>,
    field: Option<PageField>,
    warnings: &mut WarningSink,
    name: &str,
) -> Result<Vec<Inline>, DocxError> {
    if let Some(field) = field {
        skip_element(reader, b"fldSimple", name)?;
        return Ok(vec![Inline::field(field)]);
    }

    warnings.warn(
        "docx_field_degraded",
        "Unsupported DOCX fields were imported as visible text when available",
    );
    let mut inlines = Vec::new();
    loop {
        match reader.read_event().map_err(|err| xml_error(name, err))? {
            Event::Start(start) if local_name(start.name().as_ref()) == b"r" => {
                let run = parse_run(reader, None, warnings, name)?;
                append_inlines(&mut inlines, run);
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"fldSimple" => break,
            Event::Start(start) => {
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, name)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(inlines)
}

fn page_field_from_instruction(instruction: Option<&str>) -> Option<PageField> {
    let instruction = instruction?;
    let first_token = instruction
        .trim()
        .split(|ch: char| !ch.is_ascii_alphabetic())
        .find(|token| !token.is_empty())?
        .to_ascii_uppercase();
    match first_token.as_str() {
        "PAGE" => Some(PageField::PageNumber),
        "NUMPAGES" => Some(PageField::PageCount),
        "DATE" => Some(PageField::Date),
        _ => None,
    }
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

fn append_inline_content(target: &mut Vec<ParagraphContent>, source: Vec<Inline>) {
    append_paragraph_content(
        target,
        source
            .into_iter()
            .map(|inline| ParagraphContent::Inline(Box::new(inline)))
            .collect(),
    );
}

fn append_paragraph_content(target: &mut Vec<ParagraphContent>, source: Vec<ParagraphContent>) {
    for item in source {
        match item {
            ParagraphContent::Inline(inline) => {
                let inline = *inline;
                if inline.text.is_empty() {
                    continue;
                }
                let mut merged = false;
                if let Some(ParagraphContent::Inline(previous)) = target.last_mut() {
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
                        merged = true;
                    }
                }
                if !merged {
                    target.push(ParagraphContent::Inline(Box::new(inline)));
                }
            }
            ParagraphContent::Image(image) => target.push(ParagraphContent::Image(image)),
        }
    }
}

fn paragraph_content_to_blocks(
    content: Vec<ParagraphContent>,
    properties: &ParagraphProperties,
) -> Vec<ParsedBlock> {
    let mut blocks = Vec::new();
    let mut inlines = Vec::new();
    for item in content {
        match item {
            ParagraphContent::Inline(inline) => inlines.push(*inline),
            ParagraphContent::Image(image) => {
                flush_paragraph_block(&mut blocks, &mut inlines, properties);
                blocks.push(ParsedBlock {
                    block: Block::Image(image),
                    list_marker: None,
                });
            }
        }
    }
    flush_paragraph_block(&mut blocks, &mut inlines, properties);
    if blocks.is_empty() {
        blocks.push(ParsedBlock {
            block: paragraph_block_from_inlines(Vec::new(), properties.heading_level),
            list_marker: properties.list_marker,
        });
    }
    blocks
}

fn flush_paragraph_block(
    blocks: &mut Vec<ParsedBlock>,
    inlines: &mut Vec<Inline>,
    properties: &ParagraphProperties,
) {
    if inlines.is_empty() {
        return;
    }
    blocks.push(ParsedBlock {
        block: paragraph_block_from_inlines(std::mem::take(inlines), properties.heading_level),
        list_marker: properties.list_marker,
    });
}

fn paragraph_block_from_inlines(inlines: Vec<Inline>, heading_level: Option<u8>) -> Block {
    if let Some(level) = heading_level {
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

fn render_content_types_xml(
    page_regions: &DocxPageRegionExports,
    images: &DocxImageExports,
) -> String {
    let mut output = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>"#
        .to_string();
    let image_defaults = images
        .parts
        .iter()
        .map(|part| (image_extension(part.media_type), part.media_type))
        .collect::<BTreeMap<_, _>>();
    for (extension, media_type) in image_defaults {
        output.push_str("\n  <Default Extension=\"");
        output.push_str(extension);
        output.push_str("\" ContentType=\"");
        output.push_str(media_type);
        output.push_str("\"/>");
    }
    output.push_str(
        r#"
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/>"#
    );
    for part in &page_regions.parts {
        output.push_str("\n  <Override PartName=\"/");
        output.push_str(part.path);
        output.push_str("\" ContentType=\"");
        output.push_str(match part.kind {
            PageRegionPartKind::Header => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"
            }
            PageRegionPartKind::Footer => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"
            }
        });
        output.push_str("\"/>");
    }
    output.push_str("\n</Types>");
    output
}

fn render_root_rels_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="{REL_TYPE_OFFICE_DOCUMENT}" Target="word/document.xml"/>
</Relationships>"#
    )
}

fn render_document_rels_xml(
    hyperlinks: &HyperlinkIds,
    page_regions: &DocxPageRegionExports,
    images: &DocxImageExports,
) -> String {
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
    for part in &images.parts {
        output.push_str("\n  <Relationship Id=\"");
        output.push_str(&escape_xml(&part.rel_id));
        output.push_str("\" Type=\"");
        output.push_str(REL_TYPE_IMAGE);
        output.push_str("\" Target=\"");
        output.push_str(&escape_xml(&part.target));
        output.push_str("\"/>");
    }
    for part in &page_regions.parts {
        output.push_str("\n  <Relationship Id=\"");
        output.push_str(&escape_xml(&part.rel_id));
        output.push_str("\" Type=\"");
        output.push_str(match part.kind {
            PageRegionPartKind::Header => REL_TYPE_HEADER,
            PageRegionPartKind::Footer => REL_TYPE_FOOTER,
        });
        output.push_str("\" Target=\"");
        output.push_str(part.target);
        output.push_str("\"/>");
    }
    output.push_str("\n</Relationships>");
    output
}

fn render_document_xml(
    document: &Document,
    hyperlinks: &HyperlinkIds,
    page_regions: &DocxPageRegionExports,
    images: &DocxImageExports,
) -> String {
    let mut output = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
<w:body>"#,
    );

    for section in &document.sections {
        for block in &section.blocks {
            render_block_xml(block, hyperlinks, images, &mut output);
        }
    }

    output.push_str(r#"<w:sectPr>"#);
    render_section_page_region_refs(page_regions, &mut output);
    output.push_str(
        r#"<w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr>"#,
    );
    output.push_str("</w:body></w:document>");
    output
}

fn render_section_page_region_refs(page_regions: &DocxPageRegionExports, output: &mut String) {
    let has_first = page_regions.parts.iter().any(|part| {
        matches!(
            part.reference,
            PageRegionReferenceKind::FirstHeader | PageRegionReferenceKind::FirstFooter
        )
    });
    for part in &page_regions.parts {
        match part.reference {
            PageRegionReferenceKind::DefaultHeader => {
                output.push_str("<w:headerReference w:type=\"default\" r:id=\"");
                output.push_str(&escape_xml(&part.rel_id));
                output.push_str("\"/>");
            }
            PageRegionReferenceKind::DefaultFooter => {
                output.push_str("<w:footerReference w:type=\"default\" r:id=\"");
                output.push_str(&escape_xml(&part.rel_id));
                output.push_str("\"/>");
            }
            PageRegionReferenceKind::FirstHeader => {
                output.push_str("<w:headerReference w:type=\"first\" r:id=\"");
                output.push_str(&escape_xml(&part.rel_id));
                output.push_str("\"/>");
            }
            PageRegionReferenceKind::FirstFooter => {
                output.push_str("<w:footerReference w:type=\"first\" r:id=\"");
                output.push_str(&escape_xml(&part.rel_id));
                output.push_str("\"/>");
            }
        }
    }
    if has_first {
        output.push_str("<w:titlePg/>");
    }
}

fn render_page_region_part_xml(part: &DocxPageRegionPart) -> String {
    let root = match part.kind {
        PageRegionPartKind::Header => "hdr",
        PageRegionPartKind::Footer => "ftr",
    };
    let mut output = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    output.push_str("<w:");
    output.push_str(root);
    output.push_str(
        r#" xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
    );
    if part.blocks.is_empty() {
        output.push_str("<w:p/>");
    } else {
        for block in &part.blocks {
            render_page_region_block_xml(block, &mut output);
        }
    }
    output.push_str("</w:");
    output.push_str(root);
    output.push('>');
    output
}

fn render_page_region_block_xml(block: &PageRegionBlock, output: &mut String) {
    match block {
        PageRegionBlock::Paragraph(paragraph) => {
            output.push_str("<w:p>");
            render_inlines_xml(&paragraph.inlines, &HyperlinkIds::default(), output);
            output.push_str("</w:p>");
        }
    }
}

fn render_block_xml(
    block: &Block,
    hyperlinks: &HyperlinkIds,
    images: &DocxImageExports,
    output: &mut String,
) {
    match block {
        Block::Paragraph(paragraph) => render_paragraph_xml(paragraph, None, hyperlinks, output),
        Block::Heading(heading) => render_heading_xml(heading, hyperlinks, output),
        Block::List(list) => render_list_xml(list, hyperlinks, images, output),
        Block::Table(table) => render_table_xml(table, hyperlinks, images, output),
        Block::TableOfContents(table_of_contents) => render_fallback_paragraph(
            &table_of_contents_text(table_of_contents),
            hyperlinks,
            output,
        ),
        Block::Image(image) => {
            if let Some(export) = docx_image_export_for(images, &image.asset_id) {
                render_image_xml(image, export, output);
                if let Some(caption) = image
                    .presentation
                    .caption
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    render_fallback_paragraph(caption.trim(), hyperlinks, output);
                }
            } else {
                render_fallback_paragraph(&image_fallback_text(image), hyperlinks, output);
            }
        }
        Block::PageBreak => {
            output.push_str("<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>");
        }
    }
}

fn docx_image_export_for<'a>(
    images: &'a DocxImageExports,
    asset_id: &str,
) -> Option<&'a DocxImageExport> {
    images.parts.iter().find(|part| part.asset_id == asset_id)
}

fn render_image_xml(image: &ImageBlock, export: &DocxImageExport, output: &mut String) {
    let doc_pr_id = export
        .rel_id
        .trim_start_matches("rId")
        .parse::<u32>()
        .unwrap_or(1);
    let extent = image_extent_emu(image);
    output.push_str("<w:p><w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\"><wp:extent cx=\"");
    output.push_str(&extent.to_string());
    output.push_str("\" cy=\"");
    output.push_str(&extent.to_string());
    output.push_str("\"/><wp:docPr id=\"");
    output.push_str(&doc_pr_id.to_string());
    output.push_str("\" name=\"900Word image\"");
    if let Some(alt) = image
        .alt_text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        output.push_str(" descr=\"");
        output.push_str(&escape_xml(alt));
        output.push('"');
    }
    output.push_str("/><wp:cNvGraphicFramePr><a:graphicFrameLocks noChangeAspect=\"1\"/></wp:cNvGraphicFramePr><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:pic><pic:nvPicPr><pic:cNvPr id=\"");
    output.push_str(&doc_pr_id.to_string());
    output.push_str(
        "\" name=\"900Word image\"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed=\"",
    );
    output.push_str(&escape_xml(&export.rel_id));
    output.push_str("\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"");
    output.push_str(&extent.to_string());
    output.push_str("\" cy=\"");
    output.push_str(&extent.to_string());
    output.push_str("\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>");
}

fn image_extent_emu(image: &ImageBlock) -> u32 {
    let scale = image.presentation.scale_percent.clamp(10, 400) as u32;
    914_400_u32.saturating_mul(scale) / 100
}

fn image_fallback_text(image: &ImageBlock) -> String {
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
    text
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

fn render_list_xml(
    list: &ListBlock,
    hyperlinks: &HyperlinkIds,
    images: &DocxImageExports,
    output: &mut String,
) {
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
                Block::Image(image) => {
                    render_block_xml(&Block::Image(image.clone()), hyperlinks, images, output)
                }
                _ => render_fallback_paragraph(&block_text(block), hyperlinks, output),
            }
        }
    }
}

fn render_table_xml(
    table: &Table,
    hyperlinks: &HyperlinkIds,
    images: &DocxImageExports,
    output: &mut String,
) {
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
                        Block::List(list) => render_list_xml(list, hyperlinks, images, output),
                        Block::Image(image) => render_block_xml(
                            &Block::Image(image.clone()),
                            hyperlinks,
                            images,
                            output,
                        ),
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
    if let Some(field) = inline.field {
        output.push_str("<w:fldSimple w:instr=\"");
        output.push_str(match field {
            PageField::PageNumber => " PAGE ",
            PageField::PageCount => " NUMPAGES ",
            PageField::Date => " DATE ",
        });
        output.push_str("\">");
        let mut fallback = Inline::text(field.fallback_text());
        fallback.marks = inline.marks.clone();
        render_run_xml(&fallback, output);
        output.push_str("</w:fldSimple>");
        return;
    }
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

fn collect_page_region_exports(document: &Document, first_rel_id: usize) -> DocxPageRegionExports {
    let Some(section) = document.sections.first() else {
        return DocxPageRegionExports::default();
    };
    let mut parts = Vec::new();
    let mut next_id = first_rel_id;
    push_page_region_export(
        &mut parts,
        &mut next_id,
        PageRegionReferenceKind::DefaultHeader,
        PageRegionPartKind::Header,
        "word/header1.xml",
        "header1.xml",
        &section.page_regions.header,
    );
    push_page_region_export(
        &mut parts,
        &mut next_id,
        PageRegionReferenceKind::DefaultFooter,
        PageRegionPartKind::Footer,
        "word/footer1.xml",
        "footer1.xml",
        &section.page_regions.footer,
    );
    push_page_region_export(
        &mut parts,
        &mut next_id,
        PageRegionReferenceKind::FirstHeader,
        PageRegionPartKind::Header,
        "word/header2.xml",
        "header2.xml",
        &section.page_regions.first_header,
    );
    push_page_region_export(
        &mut parts,
        &mut next_id,
        PageRegionReferenceKind::FirstFooter,
        PageRegionPartKind::Footer,
        "word/footer2.xml",
        "footer2.xml",
        &section.page_regions.first_footer,
    );
    DocxPageRegionExports { parts }
}

fn collect_docx_image_exports(
    document: &Document,
    first_rel_id: usize,
) -> (DocxImageExports, usize) {
    let mut asset_ids = Vec::new();
    let mut seen = BTreeSet::new();
    for section in &document.sections {
        collect_image_asset_ids_from_blocks(&section.blocks, &mut seen, &mut asset_ids);
    }

    let mut parts = Vec::new();
    let mut next_id = first_rel_id;
    let mut total_bytes = 0_u64;
    for asset_id in asset_ids {
        if parts.len() >= MAX_DOCX_IMAGE_PARTS {
            break;
        }
        let Some(asset) = document.assets.get(&asset_id) else {
            continue;
        };
        let Some(media_type) = detect_image_media_type(&asset.bytes) else {
            continue;
        };
        if asset.media_type != media_type || asset.byte_len != asset.bytes.len() {
            continue;
        }
        let byte_len = asset.bytes.len() as u64;
        if byte_len > PackageLimits::default().max_entry_size
            || total_bytes.saturating_add(byte_len) > MAX_DOCX_IMAGE_BYTES
        {
            continue;
        }
        let extension = image_extension(media_type);
        if extension == "bin" {
            continue;
        }
        let file_name = format!("900word-image-{}.{}", parts.len() + 1, extension);
        parts.push(DocxImageExport {
            asset_id,
            rel_id: format!("rId{next_id}"),
            path: format!("word/media/{file_name}"),
            target: format!("media/{file_name}"),
            media_type,
            bytes: asset.bytes.clone(),
        });
        total_bytes = total_bytes.saturating_add(byte_len);
        next_id += 1;
    }
    (DocxImageExports { parts }, next_id)
}

fn collect_image_asset_ids_from_blocks(
    blocks: &[Block],
    seen: &mut BTreeSet<String>,
    asset_ids: &mut Vec<String>,
) {
    for block in blocks {
        match block {
            Block::Image(image) => {
                if seen.insert(image.asset_id.clone()) {
                    asset_ids.push(image.asset_id.clone());
                }
            }
            Block::List(list) => {
                for item in &list.items {
                    collect_image_asset_ids_from_blocks(&item.blocks, seen, asset_ids);
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_image_asset_ids_from_blocks(&cell.blocks, seen, asset_ids);
                    }
                }
            }
            Block::Paragraph(_)
            | Block::Heading(_)
            | Block::TableOfContents(_)
            | Block::PageBreak => {}
        }
    }
}

fn push_page_region_export(
    parts: &mut Vec<DocxPageRegionPart>,
    next_id: &mut usize,
    reference: PageRegionReferenceKind,
    kind: PageRegionPartKind,
    path: &'static str,
    target: &'static str,
    region: &PageRegion,
) {
    if !region.has_content() {
        return;
    }
    parts.push(DocxPageRegionPart {
        reference,
        kind,
        rel_id: format!("rId{next_id}"),
        path,
        target,
        blocks: region.blocks.clone(),
    });
    *next_id += 1;
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

fn resolve_page_region_target(target: &str, kind: PageRegionPartKind) -> Option<String> {
    let trimmed = target.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || trimmed.contains('\\')
        || trimmed.contains(':')
    {
        return None;
    }
    let combined = if trimmed.starts_with("word/") {
        trimmed.to_string()
    } else {
        format!("word/{trimmed}")
    };
    if combined
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return None;
    }
    let lower = combined.to_ascii_lowercase();
    let file_name = lower.strip_prefix("word/")?;
    if file_name.contains('/') {
        return None;
    }
    let matches_kind = match kind {
        PageRegionPartKind::Header => file_name.starts_with("header"),
        PageRegionPartKind::Footer => file_name.starts_with("footer"),
    };
    if matches_kind
        && lower.ends_with(".xml")
        && validate_entry_path(&combined, PackageLimits::default()).is_ok()
    {
        Some(combined)
    } else {
        None
    }
}

fn resolve_image_target(target: &str) -> Option<(String, &'static str)> {
    let trimmed = target.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || trimmed.contains('\\')
        || trimmed.contains(':')
    {
        return None;
    }
    let combined = if trimmed.starts_with("word/") {
        trimmed.to_string()
    } else {
        format!("word/{trimmed}")
    };
    if combined
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return None;
    }
    let lower = combined.to_ascii_lowercase();
    let file_name = lower.strip_prefix("word/media/")?;
    if file_name.contains('/') {
        return None;
    }
    let media_type = media_type_from_image_extension(file_name)?;
    if validate_entry_path(&combined, PackageLimits::default()).is_ok() {
        Some((combined, media_type))
    } else {
        None
    }
}

fn media_type_from_image_extension(file_name: &str) -> Option<&'static str> {
    if file_name.ends_with(".png") {
        Some("image/png")
    } else if file_name.ends_with(".jpg") || file_name.ends_with(".jpeg") {
        Some("image/jpeg")
    } else if file_name.ends_with(".gif") {
        Some("image/gif")
    } else if file_name.ends_with(".webp") {
        Some("image/webp")
    } else {
        None
    }
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

fn image_extension(media_type: &str) -> &'static str {
    match media_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "bin",
    }
}

fn generic_docx_image_id(index: usize, media_type: &str) -> String {
    format!("docx-image-{index}.{}", image_extension(media_type))
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

fn target_mode_is_external(value: Option<&str>) -> bool {
    value
        .map(|value| value.eq_ignore_ascii_case("External"))
        .unwrap_or(false)
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

    const SAMPLE_PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1,
        13, 10, 45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

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
    fn imports_synthetic_docx_header_footer_text_and_page_fields() {
        let bytes = synthetic_docx_with_parts(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<w:body>
  <w:p><w:r><w:t>Body</w:t></w:r></w:p>
  <w:sectPr>
    <w:headerReference w:type="default" r:id="rHdr1"/>
    <w:footerReference w:type="default" r:id="rFtr1"/>
    <w:headerReference w:type="first" r:id="rHdr2"/>
    <w:footerReference w:type="first" r:id="rFtr2"/>
    <w:titlePg/>
  </w:sectPr>
</w:body></w:document>"#,
            Some(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rHdr1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/>
<Relationship Id="rFtr1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/>
<Relationship Id="rHdr2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header2.xml"/>
<Relationship Id="rFtr2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer2.xml"/>
</Relationships>"#,
            ),
            None,
            &[
                (
                    "word/header1.xml",
                    r#"<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p><w:r><w:t>Page </w:t></w:r><w:fldSimple w:instr=" PAGE "><w:r><w:t>1</w:t></w:r></w:fldSimple></w:p></w:hdr>"#,
                ),
                (
                    "word/footer1.xml",
                    r#"<w:ftr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p><w:r><w:t>Total </w:t></w:r><w:fldSimple w:instr=" NUMPAGES "/></w:p></w:ftr>"#,
                ),
                (
                    "word/header2.xml",
                    r#"<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p><w:r><w:t>First </w:t></w:r><w:fldSimple w:instr=" DATE "/></w:p></w:hdr>"#,
                ),
                (
                    "word/footer2.xml",
                    r#"<w:ftr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p><w:r><w:rPr><w:b/></w:rPr><w:t>First footer</w:t></w:r></w:p></w:ftr>"#,
                ),
            ],
        );

        let document = read_docx_bytes(&bytes).expect("docx should import page regions");
        let regions = &document.sections[0].page_regions;

        assert!(regions.different_first_page);
        let PageRegionBlock::Paragraph(header) = &regions.header.blocks[0];
        assert_eq!(header.inlines[0].text, "Page ");
        assert_eq!(header.inlines[1].field, Some(PageField::PageNumber));
        let PageRegionBlock::Paragraph(footer) = &regions.footer.blocks[0];
        assert_eq!(footer.inlines[1].field, Some(PageField::PageCount));
        let PageRegionBlock::Paragraph(first_header) = &regions.first_header.blocks[0];
        assert_eq!(first_header.inlines[1].field, Some(PageField::Date));
        let PageRegionBlock::Paragraph(first_footer) = &regions.first_footer.blocks[0];
        assert_eq!(first_footer.inlines[0].marks, vec![InlineMark::Bold]);
    }

    #[test]
    fn exports_and_imports_word_core_page_regions_through_docx_converter() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: ParagraphFormat::default(),
            inlines: vec![Inline::text("Body")],
        })];
        document.sections[0].page_regions.header.blocks =
            vec![PageRegionBlock::Paragraph(PageRegionParagraph {
                inlines: vec![Inline::text("Page "), Inline::field(PageField::PageNumber)],
            })];
        document.sections[0].page_regions.footer.blocks =
            vec![PageRegionBlock::Paragraph(PageRegionParagraph {
                inlines: vec![Inline::text("of "), Inline::field(PageField::PageCount)],
            })];
        document.sections[0].page_regions.first_header.blocks =
            vec![PageRegionBlock::Paragraph(PageRegionParagraph {
                inlines: vec![Inline::text("Date "), Inline::field(PageField::Date)],
            })];
        document.sections[0].page_regions.different_first_page = true;

        let bytes = write_docx_bytes(&document).expect("docx should write page regions");
        validate_docx_package(&bytes, PackageLimits::default()).expect("written package validates");
        let mut archive =
            ZipArchive::new(Cursor::new(bytes.as_slice())).expect("written docx should open");
        let mut content_types = String::new();
        archive
            .by_name("[Content_Types].xml")
            .expect("content types should exist")
            .read_to_string(&mut content_types)
            .expect("content types should read");
        let mut document_rels = String::new();
        archive
            .by_name(DOCUMENT_RELS)
            .expect("document rels should exist")
            .read_to_string(&mut document_rels)
            .expect("document rels should read");
        let mut document_xml = String::new();
        archive
            .by_name(DOCUMENT_XML)
            .expect("document xml should exist")
            .read_to_string(&mut document_xml)
            .expect("document xml should read");
        archive
            .by_name("word/header1.xml")
            .expect("default header part should exist");
        archive
            .by_name("word/footer1.xml")
            .expect("default footer part should exist");
        archive
            .by_name("word/header2.xml")
            .expect("first header part should exist");
        assert!(content_types.contains("/word/header1.xml"));
        assert!(content_types.contains("/word/footer1.xml"));
        assert!(content_types.contains("/word/header2.xml"));
        assert!(document_rels.contains("relationships/header"));
        assert!(document_rels.contains("Target=\"header1.xml\""));
        assert!(document_rels.contains("Target=\"footer1.xml\""));
        assert!(document_xml.contains("<w:headerReference w:type=\"default\""));
        assert!(document_xml.contains("<w:footerReference w:type=\"default\""));
        assert!(document_xml.contains("<w:headerReference w:type=\"first\""));
        assert!(document_xml.contains("<w:titlePg/>"));

        let parsed = read_docx_bytes(&bytes).expect("written package should import");
        let regions = &parsed.sections[0].page_regions;

        assert!(regions.different_first_page);
        let PageRegionBlock::Paragraph(header) = &regions.header.blocks[0];
        assert_eq!(header.inlines[1].field, Some(PageField::PageNumber));
        let PageRegionBlock::Paragraph(footer) = &regions.footer.blocks[0];
        assert_eq!(footer.inlines[1].field, Some(PageField::PageCount));
        let PageRegionBlock::Paragraph(first_header) = &regions.first_header.blocks[0];
        assert_eq!(first_header.inlines[1].field, Some(PageField::Date));
    }

    #[test]
    fn ignores_unsafe_header_relationship_targets_with_generic_warning() {
        for (target, target_mode) in [
            ("../private/header1.xml", ""),
            ("/absolute/header1.xml", ""),
            ("C:/placeholder/header1.xml", ""),
            ("folder\\header1.xml", ""),
            ("https://example.invalid/header1.xml", ""),
            ("header1.xml", r#" TargetMode="External""#),
        ] {
            let rels_xml = format!(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rHdr1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="{target}"{target_mode}/>
</Relationships>"#
            );
            let bytes = synthetic_docx(
                r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<w:body>
  <w:p><w:r><w:t>Body</w:t></w:r></w:p>
  <w:sectPr><w:headerReference w:type="default" r:id="rHdr1"/></w:sectPr>
</w:body></w:document>"#,
                Some(&rels_xml),
                None,
            );

            let document =
                read_docx_bytes(&bytes).expect("unsafe page region target should degrade");

            assert!(document.sections[0].page_regions.header.blocks.is_empty());
            let warning = document
                .warnings
                .iter()
                .find(|warning| warning.code == "docx_page_region_relationship_ignored")
                .expect("unsafe relationship should warn");
            assert!(!warning.message.contains("private"));
            assert!(!warning.message.contains("header1.xml"));
            assert!(!warning.message.contains("example.invalid"));
            assert!(!warning.message.contains("C:/"));
        }
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
    fn imports_synthetic_docx_embedded_png_image() {
        let bytes = synthetic_docx_with_binary_parts(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
<w:body><w:p><w:r><w:drawing><wp:inline><wp:docPr id="1" name="Picture 1" descr="Diagram"/><a:graphic><a:graphicData><a:blip r:embed="rImg1"/></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p></w:body></w:document>"#,
            Some(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rImg1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/source-private-name.png"/>
</Relationships>"#,
            ),
            None,
            &[],
            &[("word/media/source-private-name.png", SAMPLE_PNG)],
        );

        let document = read_docx_bytes(&bytes).expect("embedded image should import");

        assert!(document.warnings.is_empty(), "{:?}", document.warnings);
        assert_eq!(document.assets.len(), 1);
        let Block::Image(image) = &document.sections[0].blocks[0] else {
            panic!("image block expected");
        };
        assert_eq!(image.asset_id, "docx-image-1.png");
        assert_eq!(image.alt_text.as_deref(), Some("Diagram"));
        let asset = document
            .assets
            .get("docx-image-1.png")
            .expect("asset should exist");
        assert_eq!(asset.media_type, "image/png");
        assert_eq!(asset.bytes, SAMPLE_PNG);
        assert_eq!(asset.original_name, None);
        assert!(!document.assets.contains_key("source-private-name.png"));
    }

    #[test]
    fn imports_docx_image_without_dropping_adjacent_text() {
        let bytes = synthetic_docx_with_binary_parts(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
<w:body><w:p><w:r><w:t>Before </w:t></w:r><w:r><w:drawing><wp:inline><wp:docPr id="1" name="Picture 1"/><a:graphic><a:graphicData><a:blip r:embed="rImg1"/></a:graphicData></a:graphic></wp:inline></w:drawing></w:r><w:r><w:t> after</w:t></w:r></w:p></w:body></w:document>"#,
            Some(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rImg1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/>
</Relationships>"#,
            ),
            None,
            &[],
            &[("word/media/image1.png", SAMPLE_PNG)],
        );

        let document = read_docx_bytes(&bytes).expect("mixed paragraph should import");

        assert_eq!(document.sections[0].blocks.len(), 3);
        let Block::Paragraph(before) = &document.sections[0].blocks[0] else {
            panic!("leading text paragraph expected");
        };
        assert_eq!(before.inlines[0].text, "Before ");
        assert!(matches!(document.sections[0].blocks[1], Block::Image(_)));
        let Block::Paragraph(after) = &document.sections[0].blocks[2] else {
            panic!("trailing text paragraph expected");
        };
        assert_eq!(after.inlines[0].text, " after");
    }

    #[test]
    fn ignores_hostile_image_relationship_targets_with_generic_warning() {
        for (target, target_mode) in [
            ("../private/image.png", ""),
            ("/absolute/image.png", ""),
            ("C:/placeholder/image.png", ""),
            ("media\\image.png", ""),
            ("https://example.invalid/image.png", ""),
            ("media/image.bmp", ""),
            ("media/image.png", r#" TargetMode="External""#),
        ] {
            let rels_xml = format!(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rImg1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="{target}"{target_mode}/>
</Relationships>"#
            );
            let bytes = synthetic_docx(
                r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
<w:body><w:p><w:r><w:drawing><wp:inline><a:graphic><a:graphicData><a:blip r:embed="rImg1"/></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p></w:body></w:document>"#,
                Some(&rels_xml),
                None,
            );

            let document =
                read_docx_bytes(&bytes).expect("unsafe image relationship should degrade");

            assert!(document.assets.is_empty());
            assert!(document.sections[0]
                .blocks
                .iter()
                .all(|block| !matches!(block, Block::Image(_))));
            let warning = document
                .warnings
                .iter()
                .find(|warning| warning.code == "docx_image_relationship_ignored")
                .expect("unsafe relationship should warn");
            assert!(!warning.message.contains("private"));
            assert!(!warning.message.contains("image.png"));
            assert!(!warning.message.contains("example.invalid"));
            assert!(!warning.message.contains("C:/"));
        }
    }

    #[test]
    fn ignores_docx_images_with_missing_or_mismatched_payloads() {
        let doc_xml = synthetic_docx_image_document(&[r#"<a:blip r:embed="rImg1"/>"#]);

        let mismatched_bytes = synthetic_docx_with_binary_parts(
            &doc_xml,
            Some(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rImg1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/not-really-jpeg.jpg"/>
</Relationships>"#,
            ),
            None,
            &[],
            &[("word/media/not-really-jpeg.jpg", SAMPLE_PNG)],
        );
        let mismatched =
            read_docx_bytes(&mismatched_bytes).expect("mismatched image should degrade");
        assert!(mismatched.assets.is_empty());
        assert_eq!(image_block_count(&mismatched), 0);
        assert!(mismatched
            .warnings
            .iter()
            .any(|warning| warning.code == "docx_image_part_ignored"));
        assert_docx_image_warnings_are_generic(&mismatched);

        let missing_bytes = synthetic_docx(
            &doc_xml,
            Some(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rImg1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/missing.png"/>
</Relationships>"#,
            ),
            None,
        );
        let missing = read_docx_bytes(&missing_bytes).expect("missing image should degrade");
        assert!(missing.assets.is_empty());
        assert_eq!(image_block_count(&missing), 0);
        assert!(missing
            .warnings
            .iter()
            .any(|warning| warning.code == "docx_image_part_missing"));
        assert_docx_image_warnings_are_generic(&missing);
    }

    #[test]
    fn ignores_linked_only_docx_image_references() {
        let doc_xml = synthetic_docx_image_document(&[r#"<a:blip r:link="rImg1"/>"#]);
        let bytes = synthetic_docx(
            &doc_xml,
            Some(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rImg1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="https://example.invalid/image.png" TargetMode="External"/>
</Relationships>"#,
            ),
            None,
        );

        let document = read_docx_bytes(&bytes).expect("linked image should degrade");

        assert!(document.assets.is_empty());
        assert_eq!(image_block_count(&document), 0);
        assert!(document
            .warnings
            .iter()
            .any(|warning| warning.code == "docx_image_reference_ignored"));
        assert_docx_image_warnings_are_generic(&document);
    }

    #[test]
    fn bounds_imported_docx_image_payload_total_bytes() {
        let mut large_png = SAMPLE_PNG.to_vec();
        large_png.resize(PackageLimits::default().max_entry_size as usize, 0);
        let doc_xml = synthetic_docx_image_document(&[
            r#"<a:blip r:embed="rImg1"/>"#,
            r#"<a:blip r:embed="rImg2"/>"#,
            r#"<a:blip r:embed="rImg3"/>"#,
        ]);
        let bytes = synthetic_docx_with_binary_parts(
            &doc_xml,
            Some(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rImg1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/large1.png"/>
<Relationship Id="rImg2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/large2.png"/>
<Relationship Id="rImg3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/large3.png"/>
</Relationships>"#,
            ),
            None,
            &[],
            &[
                ("word/media/large1.png", large_png.as_slice()),
                ("word/media/large2.png", large_png.as_slice()),
                ("word/media/large3.png", large_png.as_slice()),
            ],
        );

        let document = read_docx_bytes(&bytes).expect("over-budget images should degrade");

        assert_eq!(document.assets.len(), 2);
        assert_eq!(image_block_count(&document), 2);
        assert!(document
            .warnings
            .iter()
            .any(|warning| warning.code == "docx_image_part_ignored"));
        assert_docx_image_warnings_are_generic(&document);
    }

    #[test]
    fn bounds_imported_docx_image_relationship_count() {
        let ids = (1..=65)
            .map(|index| format!("rImg{index}"))
            .collect::<Vec<_>>();
        let blips = ids
            .iter()
            .map(|id| format!(r#"<a:blip r:embed="{id}"/>"#))
            .collect::<Vec<_>>();
        let blip_refs = blips.iter().map(String::as_str).collect::<Vec<_>>();
        let doc_xml = synthetic_docx_image_document(&blip_refs);
        let mut rels_xml = String::from(
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
        );
        for index in 1..=65 {
            rels_xml.push_str(&format!(
                r#"<Relationship Id="rImg{index}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image{index}.png"/>"#
            ));
        }
        rels_xml.push_str("</Relationships>");
        let paths = (1..=65)
            .map(|index| format!("word/media/image{index}.png"))
            .collect::<Vec<_>>();
        let binary_parts = paths
            .iter()
            .map(|path| (path.as_str(), SAMPLE_PNG))
            .collect::<Vec<_>>();
        let bytes =
            synthetic_docx_with_binary_parts(&doc_xml, Some(&rels_xml), None, &[], &binary_parts);

        let document = read_docx_bytes(&bytes).expect("excess images should degrade");

        assert_eq!(document.assets.len(), MAX_DOCX_IMAGE_PARTS);
        assert_eq!(image_block_count(&document), MAX_DOCX_IMAGE_PARTS);
        assert!(document
            .warnings
            .iter()
            .any(|warning| warning.code == "docx_too_many_images"));
        assert_docx_image_warnings_are_generic(&document);
    }

    #[test]
    fn exports_and_imports_word_core_image_assets_through_docx_converter() {
        let mut document = Document::new_untitled();
        document.assets.insert(
            "image-1.png".to_string(),
            AssetRef {
                id: "image-1.png".to_string(),
                media_type: "image/png".to_string(),
                byte_len: SAMPLE_PNG.len(),
                bytes: SAMPLE_PNG.to_vec(),
                original_name: None,
            },
        );
        document.sections[0].blocks = vec![Block::Image(ImageBlock {
            asset_id: "image-1.png".to_string(),
            presentation: ImagePresentation::default(),
            alt_text: Some("Alt text".to_string()),
        })];

        let bytes = write_docx_bytes(&document).expect("docx should write image");
        validate_docx_package(&bytes, PackageLimits::default()).expect("written package validates");
        let parsed = read_docx_bytes(&bytes).expect("written package should import");

        assert_eq!(parsed.assets.len(), 1);
        let Block::Image(image) = &parsed.sections[0].blocks[0] else {
            panic!("image should round-trip through docx converter");
        };
        assert_eq!(image.asset_id, "docx-image-1.png");
        assert_eq!(image.alt_text.as_deref(), Some("Alt text"));
        let asset = parsed
            .assets
            .get("docx-image-1.png")
            .expect("round-tripped asset should exist");
        assert_eq!(asset.media_type, "image/png");
        assert_eq!(asset.bytes, SAMPLE_PNG);
        assert_eq!(asset.original_name, None);
    }

    #[test]
    fn exports_docx_image_package_parts_relationships_and_drawing_reference() {
        let mut document = Document::new_untitled();
        document.assets.insert(
            "image-1.png".to_string(),
            AssetRef {
                id: "image-1.png".to_string(),
                media_type: "image/png".to_string(),
                byte_len: SAMPLE_PNG.len(),
                bytes: SAMPLE_PNG.to_vec(),
                original_name: None,
            },
        );
        document.sections[0].blocks = vec![Block::Image(ImageBlock {
            asset_id: "image-1.png".to_string(),
            presentation: ImagePresentation {
                caption: Some("Visible caption".to_string()),
                ..ImagePresentation::default()
            },
            alt_text: Some("Alt text".to_string()),
        })];

        let bytes = write_docx_bytes(&document).expect("docx should write image package");
        let mut archive =
            ZipArchive::new(Cursor::new(bytes.as_slice())).expect("written docx should open");
        let mut content_types = String::new();
        archive
            .by_name("[Content_Types].xml")
            .expect("content types should exist")
            .read_to_string(&mut content_types)
            .expect("content types should read");
        let mut document_rels = String::new();
        archive
            .by_name(DOCUMENT_RELS)
            .expect("document rels should exist")
            .read_to_string(&mut document_rels)
            .expect("document rels should read");
        let mut document_xml = String::new();
        archive
            .by_name(DOCUMENT_XML)
            .expect("document xml should exist")
            .read_to_string(&mut document_xml)
            .expect("document xml should read");
        let mut media = Vec::new();
        archive
            .by_name("word/media/900word-image-1.png")
            .expect("image media part should exist")
            .read_to_end(&mut media)
            .expect("image media should read");

        assert!(content_types.contains(r#"<Default Extension="png" ContentType="image/png"/>"#));
        assert!(document_rels.contains("relationships/image"));
        assert!(document_rels.contains(r#"Target="media/900word-image-1.png""#));
        assert!(document_xml.contains("<w:drawing>"));
        assert!(document_xml.contains("r:embed=\"rId3\""));
        assert!(document_xml.contains("descr=\"Alt text\""));
        assert!(document_xml.contains("Visible caption"));
        assert_eq!(media, SAMPLE_PNG);
    }

    #[test]
    fn skips_oversized_docx_image_exports_with_visible_fallback_text() {
        let mut oversized_png = SAMPLE_PNG.to_vec();
        oversized_png.resize(PackageLimits::default().max_entry_size as usize + 1, 0);
        let mut document = Document::new_untitled();
        document.assets.insert(
            "image-large.png".to_string(),
            AssetRef {
                id: "image-large.png".to_string(),
                media_type: "image/png".to_string(),
                byte_len: oversized_png.len(),
                bytes: oversized_png,
                original_name: None,
            },
        );
        document.sections[0].blocks = vec![Block::Image(ImageBlock {
            asset_id: "image-large.png".to_string(),
            presentation: ImagePresentation {
                caption: Some("Oversized caption".to_string()),
                ..ImagePresentation::default()
            },
            alt_text: Some("Oversized alt".to_string()),
        })];

        let bytes = write_docx_bytes(&document).expect("docx should write fallback text");
        validate_docx_package(&bytes, PackageLimits::default()).expect("written package validates");
        let mut archive =
            ZipArchive::new(Cursor::new(bytes.as_slice())).expect("written docx should open");
        assert!(matches!(
            archive.by_name("word/media/900word-image-1.png"),
            Err(zip::result::ZipError::FileNotFound)
        ));
        let mut document_xml = String::new();
        archive
            .by_name(DOCUMENT_XML)
            .expect("document xml should exist")
            .read_to_string(&mut document_xml)
            .expect("document xml should read");

        assert!(!document_xml.contains("<w:drawing>"));
        assert!(document_xml.contains("Oversized alt"));
        assert!(document_xml.contains("Oversized caption"));
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

    fn synthetic_docx_image_document(blips: &[&str]) -> String {
        let mut body = String::new();
        for blip in blips {
            body.push_str("<w:p><w:r><w:drawing><wp:inline><a:graphic><a:graphicData>");
            body.push_str(blip);
            body.push_str("</a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>");
        }
        format!(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><w:body>{body}</w:body></w:document>"#
        )
    }

    fn image_block_count(document: &Document) -> usize {
        document
            .sections
            .iter()
            .flat_map(|section| section.blocks.iter())
            .filter(|block| matches!(block, Block::Image(_)))
            .count()
    }

    fn assert_docx_image_warnings_are_generic(document: &Document) {
        for warning in &document.warnings {
            assert!(!warning.message.contains("private"));
            assert!(!warning.message.contains("media/"));
            assert!(!warning.message.contains("word/media"));
            assert!(!warning.message.contains("image.png"));
            assert!(!warning.message.contains("example.invalid"));
            assert!(!warning.message.contains("C:/"));
        }
    }

    fn synthetic_docx(
        document_xml: &str,
        rels_xml: Option<&str>,
        numbering_xml: Option<&str>,
    ) -> Vec<u8> {
        synthetic_docx_with_parts(document_xml, rels_xml, numbering_xml, &[])
    }

    fn synthetic_docx_with_parts(
        document_xml: &str,
        rels_xml: Option<&str>,
        numbering_xml: Option<&str>,
        extra_parts: &[(&str, &str)],
    ) -> Vec<u8> {
        synthetic_docx_with_binary_parts(document_xml, rels_xml, numbering_xml, extra_parts, &[])
    }

    fn synthetic_docx_with_binary_parts(
        document_xml: &str,
        rels_xml: Option<&str>,
        numbering_xml: Option<&str>,
        extra_parts: &[(&str, &str)],
        binary_parts: &[(&str, &[u8])],
    ) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer
            .start_file("[Content_Types].xml", options)
            .expect("content types should start");
        writer
            .write_all(
                render_content_types_xml(
                    &DocxPageRegionExports::default(),
                    &DocxImageExports::default(),
                )
                .as_bytes(),
            )
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
        for (path, xml) in extra_parts {
            writer
                .start_file(*path, options)
                .expect("part should start");
            writer.write_all(xml.as_bytes()).expect("part should write");
        }
        for (path, bytes) in binary_parts {
            writer
                .start_file(*path, options)
                .expect("binary part should start");
            writer.write_all(bytes).expect("binary part should write");
        }
        writer.finish().expect("zip should finish");
        cursor.into_inner()
    }
}
