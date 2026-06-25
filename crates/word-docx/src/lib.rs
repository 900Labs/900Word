use chrono::{DateTime, SecondsFormat, Utc};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};
use thiserror::Error;
use word_core::{
    collect_ordered_note_references, normalize_comment_author, sanitize_bookmark_id,
    sanitize_table_cell_background_color, sanitize_table_column_widths, validate_comment_body,
    validate_comment_id, validate_note_body, validate_note_id, validate_note_reference,
    validate_tracked_change_id, AssetRef, Block, CommentThread, Document, DocumentWarning, Heading,
    ImageBlock, ImagePresentation, Inline, InlineMark, InlineNoteReference, InlineStyle, ListBlock,
    ListItem, Note, NoteKind, PageField, PageRegion, PageRegionBlock, PageRegionParagraph,
    PageRegions, PageSetup, Paragraph, ParagraphAlignment, ParagraphFormat, StyleId, Table,
    TableCell, TableCellBorder, TableCellPresentation, TableOfContents, TableOfContentsEntry,
    TableRow, TrackedChange, TrackedChangeKind, DEFAULT_TRACKED_CHANGE_AUTHOR, MAX_NOTES,
    MAX_TABLE_WIDTH_COLUMNS,
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
const REL_TYPE_COMMENTS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments";
const REL_TYPE_FOOTNOTES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes";
const REL_TYPE_ENDNOTES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/endnotes";
const REL_TYPE_HEADER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/header";
const REL_TYPE_FOOTER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer";
const REL_TYPE_IMAGE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
const PAGE_REGION_XML: &str = "DOCX page region";
const DOCX_COMMENTS_XML: &str = "DOCX comments";
const DOCX_NOTES_XML: &str = "DOCX notes";
const DOCX_TOC_TITLE_STYLE_ID: &str = "Word900TocTitle";
const DOCX_TOC_ENTRY_STYLE_PREFIX: &str = "Word900TocEntry";
const MAX_DOCX_IMAGE_PARTS: usize = 64;
const MAX_DOCX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_DOCX_COMMENT_PARTS: usize = 4;
const MAX_DOCX_COMMENTS: usize = 128;
const MAX_DOCX_REVISIONS: usize = 512;
const DOCX_TABLE_GRID_TOTAL_DXA: u32 = 10_000;
const DOCX_LINE_SPACING_BASE: u32 = 240;
const MAX_DOCX_PARAGRAPH_SPACING_MM: u16 = 100;
const MAX_DOCX_PARAGRAPH_INDENT_MM: u16 = 100;
const MAX_DOCX_FIRST_LINE_INDENT_MM: i16 = 100;
const MIN_DOCX_LINE_SPACING_PER_MILLE: u16 = 500;
const MAX_DOCX_LINE_SPACING_PER_MILLE: u16 = 3000;
const SUPPORTED_DOCX_INLINE_FONT_SIZES_PT: &[u16] = &[9, 10, 11, 12, 14, 16, 18, 24, 32];
const IMPORTED_DOCX_REVISION_AUTHOR: &str = "External Reviewer";

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
    toc_role: Option<TocParagraphRole>,
    counts_for_cell_alignment: bool,
    paragraph_alignment: Option<ParagraphAlignment>,
}

#[derive(Debug, Clone, Default)]
struct RelationshipMap {
    hyperlinks: BTreeMap<String, String>,
    comments: BTreeMap<String, String>,
    notes: Vec<DocxNoteRelationship>,
    page_regions: BTreeMap<String, PageRegionRelationship>,
    images: BTreeMap<String, DocxImageRelationship>,
}

#[derive(Debug, Clone)]
struct DocxNoteRelationship {
    kind: NoteKind,
    target: String,
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
struct ImportedDocxComments {
    by_raw_id: BTreeMap<String, ImportedDocxComment>,
    comments: BTreeMap<String, CommentThread>,
}

#[derive(Debug, Clone)]
struct ImportedDocxComment {
    local_id: String,
}

#[derive(Debug, Clone, Default)]
struct ImportedDocxNotes {
    footnotes: BTreeMap<String, ImportedDocxNote>,
    endnotes: BTreeMap<String, ImportedDocxNote>,
}

impl ImportedDocxNotes {
    fn get(&self, kind: NoteKind, raw_id: &str) -> Option<&ImportedDocxNote> {
        match kind {
            NoteKind::Footnote => self.footnotes.get(raw_id),
            NoteKind::Endnote => self.endnotes.get(raw_id),
        }
    }

    fn len(&self) -> usize {
        self.footnotes.len() + self.endnotes.len()
    }

    fn has_unanchored(&self, referenced_raw_keys: &BTreeSet<String>) -> bool {
        self.footnotes
            .keys()
            .any(|raw_id| !referenced_raw_keys.contains(&docx_note_key(NoteKind::Footnote, raw_id)))
            || self.endnotes.keys().any(|raw_id| {
                !referenced_raw_keys.contains(&docx_note_key(NoteKind::Endnote, raw_id))
            })
    }

    fn insert(
        &mut self,
        kind: NoteKind,
        raw_id: String,
        note: ImportedDocxNote,
    ) -> Option<ImportedDocxNote> {
        match kind {
            NoteKind::Footnote => self.footnotes.insert(raw_id, note),
            NoteKind::Endnote => self.endnotes.insert(raw_id, note),
        }
    }

    fn contains_raw_id(&self, kind: NoteKind, raw_id: &str) -> bool {
        match kind {
            NoteKind::Footnote => self.footnotes.contains_key(raw_id),
            NoteKind::Endnote => self.endnotes.contains_key(raw_id),
        }
    }
}

#[derive(Debug, Clone)]
struct ImportedDocxNote {
    body: String,
}

#[derive(Debug, Clone, Copy)]
struct DocxImportContext<'a> {
    rels: &'a RelationshipMap,
    images: &'a ImportedDocxImages,
    comments: &'a ImportedDocxComments,
    notes: &'a ImportedDocxNotes,
}

struct DocxBodyParseState<'a> {
    warnings: &'a mut WarningSink,
    anchored_comment_ids: &'a mut BTreeSet<String>,
    revisions: &'a mut RevisionImportState,
    notes: &'a mut NoteImportState,
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

#[derive(Debug, Clone, Default)]
struct DocxCommentExports {
    rel_id: Option<String>,
    ids: BTreeMap<String, u32>,
    comments: Vec<DocxCommentExport>,
}

impl DocxCommentExports {
    fn has_comments(&self) -> bool {
        !self.comments.is_empty()
    }

    fn ids_for_inline(&self, inline: &Inline) -> Vec<String> {
        if inline.text.is_empty()
            || inline.field.is_some()
            || inline.note_reference.is_some()
            || inline.tracked_change.is_some()
        {
            return Vec::new();
        }
        inline
            .comment_ids
            .iter()
            .filter(|id| self.ids.contains_key(*id))
            .cloned()
            .collect()
    }

    fn numeric_id(&self, local_id: &str) -> Option<u32> {
        self.ids.get(local_id).copied()
    }
}

#[derive(Debug, Clone)]
struct DocxCommentExport {
    numeric_id: u32,
    author: String,
    body: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
struct DocxNoteExports {
    rel_id_footnotes: Option<String>,
    rel_id_endnotes: Option<String>,
    ids: BTreeMap<String, DocxNoteExportId>,
    footnotes: Vec<DocxNoteExport>,
    endnotes: Vec<DocxNoteExport>,
}

impl DocxNoteExports {
    fn has_footnotes(&self) -> bool {
        !self.footnotes.is_empty()
    }

    fn has_endnotes(&self) -> bool {
        !self.endnotes.is_empty()
    }

    fn numeric_id(&self, reference: &InlineNoteReference) -> Option<u32> {
        self.ids
            .get(&reference.id)
            .filter(|export| export.kind == reference.kind)
            .map(|export| export.numeric_id)
    }
}

#[derive(Debug, Clone, Copy)]
struct DocxNoteExportId {
    kind: NoteKind,
    numeric_id: u32,
}

#[derive(Debug, Clone)]
struct DocxNoteExport {
    numeric_id: u32,
    body: String,
}

#[derive(Debug, Clone, Default)]
struct DocxRevisionExports {
    ids: BTreeMap<String, u32>,
}

impl DocxRevisionExports {
    fn numeric_id(&self, local_id: &str) -> Option<u32> {
        self.ids.get(local_id).copied()
    }
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
    page_setup: Option<PageSetup>,
    page_regions: PageRegions,
    anchored_comment_ids: BTreeSet<String>,
    notes: BTreeMap<String, Note>,
}

#[derive(Debug, Clone, Default)]
struct SectionProperties {
    page_setup: Option<PageSetup>,
    page_regions: PageRegionReferences,
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
    toc_role: Option<TocParagraphRole>,
    bookmark_id: Option<String>,
    format: ParagraphFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TocParagraphRole {
    Title,
    Entry(u8),
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
    strike: bool,
    double_strike: bool,
    superscript: bool,
    subscript: bool,
    style: InlineStyle,
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
        if self.strike || self.double_strike {
            marks.push(InlineMark::Strikethrough);
        }
        if self.superscript {
            marks.push(InlineMark::Superscript);
        } else if self.subscript {
            marks.push(InlineMark::Subscript);
        }
        marks
    }
}

#[derive(Debug, Clone, Default)]
struct RevisionImportState {
    imported: usize,
}

impl RevisionImportState {
    fn tracked_change(
        &mut self,
        start: &BytesStart<'_>,
        kind: TrackedChangeKind,
        warnings: &mut WarningSink,
        name: &str,
    ) -> Result<Option<TrackedChange>, DocxError> {
        if self.imported >= MAX_DOCX_REVISIONS {
            warnings.warn(
                "docx_revisions_over_limit",
                "Excess DOCX tracked changes were imported as visible text",
            );
            return Ok(None);
        }

        self.imported += 1;
        Ok(Some(TrackedChange {
            id: next_imported_docx_tracked_change_id(self.imported),
            kind,
            author: safe_imported_docx_revision_author(
                attr_value(start, b"author", name)?.as_deref(),
                warnings,
            ),
            created_at: safe_docx_revision_timestamp(attr_value(start, b"date", name)?),
        }))
    }
}

#[derive(Debug, Clone, Default)]
struct NoteImportState {
    footnotes: usize,
    endnotes: usize,
    raw_to_local: BTreeMap<String, String>,
    notes: BTreeMap<String, Note>,
}

impl NoteImportState {
    fn reference(
        &mut self,
        start: &BytesStart<'_>,
        kind: NoteKind,
        imported_notes: &ImportedDocxNotes,
        warnings: &mut WarningSink,
        name: &str,
        hidden_context: bool,
    ) -> Result<Option<InlineNoteReference>, DocxError> {
        let Some(raw_id) = attr_value(start, b"id", name)?
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        else {
            warnings.warn(
                "docx_note_reference_ignored",
                "Unsupported DOCX note references were imported as visible fallback text",
            );
            return Ok(None);
        };

        if hidden_context {
            warnings.warn(
                "docx_note_reference_ignored",
                "Unsupported DOCX note references were imported as visible fallback text",
            );
            return Ok(None);
        }

        let key = docx_note_key(kind, &raw_id);
        if self.raw_to_local.contains_key(&key) {
            warnings.warn(
                "docx_note_reference_ignored",
                "Unsupported DOCX note references were imported as visible fallback text",
            );
            return Ok(None);
        }

        let Some(note) = imported_notes.get(kind, &raw_id) else {
            warnings.warn(
                "docx_note_reference_ignored",
                "Unsupported DOCX note references were imported as visible fallback text",
            );
            return Ok(None);
        };

        if self.notes.len() >= MAX_NOTES {
            warnings.warn(
                "docx_notes_over_limit",
                "Excess DOCX notes were imported as visible fallback text",
            );
            return Ok(None);
        }

        let sequence = match kind {
            NoteKind::Footnote => {
                self.footnotes += 1;
                self.footnotes
            }
            NoteKind::Endnote => {
                self.endnotes += 1;
                self.endnotes
            }
        };
        let local_id = next_imported_docx_note_id(kind, sequence);
        let reference = InlineNoteReference {
            id: local_id.clone(),
            kind,
            label: sequence.to_string(),
        };
        if validate_note_reference(&reference).is_err() {
            warnings.warn(
                "docx_note_reference_ignored",
                "Unsupported DOCX note references were imported as visible fallback text",
            );
            return Ok(None);
        }

        self.raw_to_local.insert(key, local_id.clone());
        self.notes.insert(
            local_id.clone(),
            Note {
                id: local_id,
                kind,
                body: note.body.clone(),
            },
        );
        Ok(Some(reference))
    }

    fn referenced_raw_keys(&self) -> BTreeSet<String> {
        self.raw_to_local.keys().cloned().collect()
    }
}

#[derive(Debug, Clone, Default)]
struct CommentImportState {
    active: Vec<ActiveCommentRange>,
    completed: BTreeSet<String>,
}

impl CommentImportState {
    fn start_range(
        &mut self,
        start: &BytesStart<'_>,
        comments: &ImportedDocxComments,
        warnings: &mut WarningSink,
    ) -> Result<(), DocxError> {
        let Some(raw_id) = attr_value(start, b"id", DOCUMENT_XML)? else {
            warnings.warn(
                "docx_comment_range_ignored",
                "Unsupported DOCX comment ranges were ignored during import",
            );
            return Ok(());
        };
        let Some(comment) = comments.by_raw_id.get(&raw_id) else {
            warnings.warn(
                "docx_comment_range_ignored",
                "Unsupported DOCX comment ranges were ignored during import",
            );
            return Ok(());
        };
        if self.active.iter().any(|range| range.raw_id == raw_id) {
            warnings.warn(
                "docx_comment_range_ignored",
                "Unsupported DOCX comment ranges were ignored during import",
            );
            return Ok(());
        }
        self.active.push(ActiveCommentRange {
            raw_id,
            local_id: comment.local_id.clone(),
            saw_text: false,
        });
        Ok(())
    }

    fn end_range(
        &mut self,
        start: &BytesStart<'_>,
        warnings: &mut WarningSink,
    ) -> Result<(), DocxError> {
        let Some(raw_id) = attr_value(start, b"id", DOCUMENT_XML)? else {
            warnings.warn(
                "docx_comment_range_ignored",
                "Unsupported DOCX comment ranges were ignored during import",
            );
            return Ok(());
        };
        let Some(position) = self.active.iter().rposition(|range| range.raw_id == raw_id) else {
            warnings.warn(
                "docx_comment_range_ignored",
                "Unsupported DOCX comment ranges were ignored during import",
            );
            return Ok(());
        };
        let range = self.active.remove(position);
        if range.saw_text {
            self.completed.insert(range.local_id);
        } else {
            warnings.warn(
                "docx_comment_range_ignored",
                "Unsupported DOCX comment ranges were ignored during import",
            );
        }
        Ok(())
    }

    fn reference_marker(
        &self,
        start: &BytesStart<'_>,
        comments: &ImportedDocxComments,
        warnings: &mut WarningSink,
    ) -> Result<(), DocxError> {
        let Some(raw_id) = attr_value(start, b"id", DOCUMENT_XML)? else {
            warnings.warn(
                "docx_comment_range_ignored",
                "Unsupported DOCX comment ranges were ignored during import",
            );
            return Ok(());
        };
        let Some(comment) = comments.by_raw_id.get(&raw_id) else {
            warnings.warn(
                "docx_comment_range_ignored",
                "Unsupported DOCX comment ranges were ignored during import",
            );
            return Ok(());
        };
        if self.active.iter().any(|range| range.raw_id == raw_id)
            || self.completed.contains(&comment.local_id)
        {
            return Ok(());
        }
        warnings.warn(
            "docx_comment_range_ignored",
            "Unsupported DOCX comment ranges were ignored during import",
        );
        Ok(())
    }

    fn mark_visible_text(&mut self) {
        for range in &mut self.active {
            range.saw_text = true;
        }
    }

    fn active_local_ids(&self) -> Vec<String> {
        self.active
            .iter()
            .map(|range| range.local_id.clone())
            .collect()
    }

    fn finish_paragraph(
        self,
        anchored_comment_ids: &mut BTreeSet<String>,
        warnings: &mut WarningSink,
    ) {
        if !self.active.is_empty() {
            warnings.warn(
                "docx_comment_range_ignored",
                "Unsupported DOCX comment ranges were ignored during import",
            );
        }
        anchored_comment_ids.extend(self.completed);
    }
}

#[derive(Debug, Clone)]
struct ActiveCommentRange {
    raw_id: String,
    local_id: String,
    saw_text: bool,
}

#[derive(Debug, Clone)]
struct HyperlinkRef {
    href: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct HyperlinkIds {
    external: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default)]
struct DocxBookmarkExports {
    ids: BTreeMap<String, u32>,
}

impl DocxBookmarkExports {
    fn numeric_id(&self, bookmark_id: Option<&str>) -> Option<u32> {
        let bookmark_id = bookmark_id.and_then(sanitize_bookmark_id)?;
        self.ids.get(&bookmark_id).copied()
    }
}

struct DocxRenderContext<'a> {
    hyperlinks: &'a HyperlinkIds,
    bookmarks: &'a DocxBookmarkExports,
    images: &'a DocxImageExports,
    comments: &'a DocxCommentExports,
    notes: &'a DocxNoteExports,
    revisions: &'a DocxRevisionExports,
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
    let comments_part_xml = read_comment_parts(&mut archive, &rels, &mut warnings)?;
    let imported_comments = parse_docx_comments(&comments_part_xml, &mut warnings)?;
    let note_part_xml = read_note_parts(&mut archive, &rels, &mut warnings)?;
    let imported_notes = parse_docx_notes(&note_part_xml, &mut warnings)?;
    let numbering = if numbering_xml.is_empty() {
        NumberingMap::default()
    } else {
        parse_numbering_xml(&numbering_xml, &mut warnings)?
    };
    let context = DocxImportContext {
        rels: &rels,
        images: &imported_images,
        comments: &imported_comments,
        notes: &imported_notes,
    };
    let parsed_document = parse_document_xml(
        &document_xml,
        &context,
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
        if let Some(page_setup) = parsed_document.page_setup {
            section.page = page_setup;
        }
        section.page_regions = parsed_document.page_regions;
    }
    document.warnings = warnings.warnings;
    document.comments = imported_comments
        .comments
        .into_iter()
        .filter(|(id, _)| parsed_document.anchored_comment_ids.contains(id))
        .collect();
    document.notes = parsed_document.notes;
    prune_comment_ids_from_blocks(
        &mut document.sections[0].blocks,
        &parsed_document.anchored_comment_ids,
    );
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
    let bookmark_exports = collect_docx_bookmark_exports(document);
    let (image_exports, next_rel_id) = collect_docx_image_exports(document, hyperlinks.len() + 3);
    let page_region_exports = collect_page_region_exports(document, next_rel_id);
    let next_rel_id = next_rel_id + page_region_exports.parts.len();
    let comment_exports = collect_docx_comment_exports(document, next_rel_id);
    let next_rel_id = next_rel_id + usize::from(comment_exports.has_comments());
    let note_exports = collect_docx_note_exports(document, next_rel_id);
    let revision_exports = collect_docx_revision_exports(document);
    let compressed = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);

    writer.start_file("[Content_Types].xml", compressed)?;
    writer.write_all(
        render_content_types_xml(
            &page_region_exports,
            &image_exports,
            &comment_exports,
            &note_exports,
        )
        .as_bytes(),
    )?;

    writer.start_file("_rels/.rels", compressed)?;
    writer.write_all(render_root_rels_xml().as_bytes())?;

    writer.start_file(DOCUMENT_XML, compressed)?;
    let render_context = DocxRenderContext {
        hyperlinks: &hyperlink_ids,
        bookmarks: &bookmark_exports,
        images: &image_exports,
        comments: &comment_exports,
        notes: &note_exports,
        revisions: &revision_exports,
    };
    writer.write_all(
        render_document_xml(document, &render_context, &page_region_exports).as_bytes(),
    )?;

    writer.start_file(DOCUMENT_RELS, compressed)?;
    writer.write_all(
        render_document_rels_xml(
            &hyperlink_ids,
            &page_region_exports,
            &image_exports,
            &comment_exports,
            &note_exports,
        )
        .as_bytes(),
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
    if comment_exports.has_comments() {
        writer.start_file("word/comments.xml", compressed)?;
        writer.write_all(render_comments_xml(&comment_exports).as_bytes())?;
    }
    if note_exports.has_footnotes() {
        writer.start_file("word/footnotes.xml", compressed)?;
        writer
            .write_all(render_notes_xml(NoteKind::Footnote, &note_exports.footnotes).as_bytes())?;
    }
    if note_exports.has_endnotes() {
        writer.start_file("word/endnotes.xml", compressed)?;
        writer.write_all(render_notes_xml(NoteKind::Endnote, &note_exports.endnotes).as_bytes())?;
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
                    (Some(id), Some(rel_type), Some(target)) if rel_type == REL_TYPE_COMMENTS => {
                        if target_mode_is_external(target_mode.as_deref()) {
                            warnings.warn(
                                "docx_comments_relationship_ignored",
                                "Unsupported DOCX comments relationships were ignored during import",
                            );
                            continue;
                        }
                        if relationships.comments.len() >= MAX_DOCX_COMMENT_PARTS {
                            warnings.warn(
                                "docx_comments_relationship_ignored",
                                "Unsupported DOCX comments relationships were ignored during import",
                            );
                            continue;
                        }
                        if let Some(target) = resolve_comments_target(&target) {
                            relationships.comments.insert(id, target);
                        } else {
                            warnings.warn(
                                "docx_comments_relationship_ignored",
                                "Unsupported DOCX comments relationships were ignored during import",
                            );
                        }
                    }
                    (Some(_id), Some(rel_type), Some(target))
                        if rel_type == REL_TYPE_FOOTNOTES || rel_type == REL_TYPE_ENDNOTES =>
                    {
                        if target_mode_is_external(target_mode.as_deref()) {
                            warnings.warn(
                                "docx_notes_relationship_ignored",
                                "Unsupported DOCX note relationships were ignored during import",
                            );
                            continue;
                        }
                        let kind = if rel_type == REL_TYPE_FOOTNOTES {
                            NoteKind::Footnote
                        } else {
                            NoteKind::Endnote
                        };
                        if relationships.notes.iter().any(|note| note.kind == kind) {
                            warnings.warn(
                                "docx_notes_relationship_ignored",
                                "Unsupported DOCX note relationships were ignored during import",
                            );
                            continue;
                        }
                        if let Some(target) = resolve_note_target(&target, kind) {
                            relationships
                                .notes
                                .push(DocxNoteRelationship { kind, target });
                        } else {
                            warnings.warn(
                                "docx_notes_relationship_ignored",
                                "Unsupported DOCX note relationships were ignored during import",
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

fn read_comment_parts<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    rels: &RelationshipMap,
    warnings: &mut WarningSink,
) -> Result<BTreeMap<String, String>, DocxError> {
    let mut parts = BTreeMap::new();
    for target in rels.comments.values() {
        if parts.contains_key(target) {
            continue;
        }
        match archive.by_name(target) {
            Ok(mut file) => {
                let mut xml = String::new();
                file.read_to_string(&mut xml)?;
                parts.insert(target.clone(), xml);
            }
            Err(zip::result::ZipError::FileNotFound) => {
                warnings.warn(
                    "docx_comments_part_missing",
                    "DOCX comments with missing content were ignored during import",
                );
            }
            Err(err) => return Err(err.into()),
        }
    }
    Ok(parts)
}

#[derive(Debug, Clone)]
struct DocxNotePartXml {
    kind: NoteKind,
    xml: String,
}

fn read_note_parts<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    rels: &RelationshipMap,
    warnings: &mut WarningSink,
) -> Result<Vec<DocxNotePartXml>, DocxError> {
    let mut parts = Vec::new();
    let mut seen = BTreeSet::new();
    for relationship in &rels.notes {
        if !seen.insert(relationship.target.clone()) {
            continue;
        }
        match archive.by_name(&relationship.target) {
            Ok(mut file) => {
                let mut xml = String::new();
                file.read_to_string(&mut xml)?;
                parts.push(DocxNotePartXml {
                    kind: relationship.kind,
                    xml,
                });
            }
            Err(zip::result::ZipError::FileNotFound) => {
                warnings.warn(
                    "docx_notes_part_missing",
                    "DOCX notes with missing content were ignored during import",
                );
            }
            Err(err) => return Err(err.into()),
        }
    }
    Ok(parts)
}

fn parse_docx_notes(
    parts: &[DocxNotePartXml],
    warnings: &mut WarningSink,
) -> Result<ImportedDocxNotes, DocxError> {
    let mut imported = ImportedDocxNotes::default();
    for part in parts {
        parse_docx_note_part_xml(&part.xml, part.kind, &mut imported, warnings)?;
    }
    Ok(imported)
}

fn parse_docx_note_part_xml(
    xml: &str,
    kind: NoteKind,
    imported: &mut ImportedDocxNotes,
    warnings: &mut WarningSink,
) -> Result<(), DocxError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let note_element = docx_note_element_name(kind);

    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCX_NOTES_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == note_element => {
                if is_docx_note_separator(&start)? {
                    skip_element(&mut reader, note_element, DOCX_NOTES_XML)?;
                    continue;
                }
                if imported.len() >= MAX_NOTES {
                    warnings.warn(
                        "docx_notes_over_limit",
                        "Excess DOCX notes were imported as visible fallback text",
                    );
                    skip_element(&mut reader, note_element, DOCX_NOTES_XML)?;
                    continue;
                }
                parse_docx_note(&mut reader, &start, kind, imported, warnings)?;
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == note_element => {
                if !is_docx_note_separator(&start)? {
                    warnings.warn(
                        "docx_note_ignored",
                        "Unsupported DOCX notes were ignored during import",
                    );
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(())
}

fn parse_docx_note(
    reader: &mut Reader<&[u8]>,
    start: &BytesStart<'_>,
    kind: NoteKind,
    imported: &mut ImportedDocxNotes,
    warnings: &mut WarningSink,
) -> Result<(), DocxError> {
    let note_element = docx_note_element_name(kind);
    let Some(raw_id) = attr_value(start, b"id", DOCX_NOTES_XML)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        warnings.warn(
            "docx_note_ignored",
            "Unsupported DOCX notes were ignored during import",
        );
        skip_element(reader, note_element, DOCX_NOTES_XML)?;
        return Ok(());
    };
    if imported.contains_raw_id(kind, &raw_id) {
        warnings.warn(
            "docx_note_ignored",
            "Unsupported DOCX notes were ignored during import",
        );
        skip_element(reader, note_element, DOCX_NOTES_XML)?;
        return Ok(());
    }

    let mut paragraphs = Vec::new();
    let mut degraded = false;
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCX_NOTES_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"p" => {
                let parsed = parse_docx_note_paragraph(reader, warnings)?;
                degraded |= parsed.degraded;
                paragraphs.push(parsed.text);
            }
            Event::End(end) if local_name(end.name().as_ref()) == note_element => break,
            Event::Start(start) => {
                degraded = true;
                warnings.warn(
                    "docx_note_content_degraded",
                    "Unsupported DOCX note content was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCX_NOTES_XML)?;
            }
            Event::Empty(_) => {
                degraded = true;
                warnings.warn(
                    "docx_note_content_degraded",
                    "Unsupported DOCX note content was ignored during import",
                );
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if degraded {
        warnings.warn(
            "docx_note_ignored",
            "Unsupported DOCX notes were ignored during import",
        );
        return Ok(());
    }

    let body = paragraphs
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let Ok(body) = validate_note_body(&body) else {
        warnings.warn(
            "docx_note_ignored",
            "Unsupported DOCX notes were ignored during import",
        );
        return Ok(());
    };
    imported.insert(kind, raw_id, ImportedDocxNote { body });
    Ok(())
}

#[derive(Debug, Clone)]
struct ParsedDocxNoteText {
    text: String,
    degraded: bool,
}

fn parse_docx_note_paragraph(
    reader: &mut Reader<&[u8]>,
    warnings: &mut WarningSink,
) -> Result<ParsedDocxNoteText, DocxError> {
    let mut text = String::new();
    let mut degraded = false;
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCX_NOTES_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"pPr" => {
                skip_element(reader, b"pPr", DOCX_NOTES_XML)?;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"r" => {
                let parsed = parse_docx_note_run(reader, warnings)?;
                degraded |= parsed.degraded;
                text.push_str(&parsed.text);
            }
            Event::Start(start)
                if is_any_docx_revision_markup(local_name(start.name().as_ref())) =>
            {
                degraded = true;
                warnings.warn(
                    "docx_note_content_degraded",
                    "Unsupported DOCX note content was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCX_NOTES_XML)?;
            }
            Event::Empty(start)
                if is_any_docx_revision_markup(local_name(start.name().as_ref())) =>
            {
                degraded = true;
                warnings.warn(
                    "docx_note_content_degraded",
                    "Unsupported DOCX note content was ignored during import",
                );
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"p" => break,
            Event::Start(start) => {
                degraded = true;
                warnings.warn(
                    "docx_note_content_degraded",
                    "Unsupported DOCX note content was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCX_NOTES_XML)?;
            }
            Event::Empty(_) => {
                degraded = true;
                warnings.warn(
                    "docx_note_content_degraded",
                    "Unsupported DOCX note content was ignored during import",
                );
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(ParsedDocxNoteText { text, degraded })
}

fn parse_docx_note_run(
    reader: &mut Reader<&[u8]>,
    warnings: &mut WarningSink,
) -> Result<ParsedDocxNoteText, DocxError> {
    let mut text = String::new();
    let mut degraded = false;
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCX_NOTES_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"rPr" => {
                skip_element(reader, b"rPr", DOCX_NOTES_XML)?;
            }
            Event::Start(start)
                if matches!(local_name(start.name().as_ref()), b"t" | b"delText") =>
            {
                let end = local_name(start.name().as_ref()).to_vec();
                text.push_str(&read_text_element(reader, &end, DOCX_NOTES_XML)?);
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
                    b"footnoteRef" | b"endnoteRef"
                ) => {}
            Event::Start(start)
                if matches!(
                    local_name(start.name().as_ref()),
                    b"footnoteRef" | b"endnoteRef"
                ) =>
            {
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCX_NOTES_XML)?;
            }
            Event::Start(start)
                if is_any_docx_revision_markup(local_name(start.name().as_ref())) =>
            {
                degraded = true;
                warnings.warn(
                    "docx_note_content_degraded",
                    "Unsupported DOCX note content was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCX_NOTES_XML)?;
            }
            Event::Empty(start)
                if is_any_docx_revision_markup(local_name(start.name().as_ref())) =>
            {
                degraded = true;
                warnings.warn(
                    "docx_note_content_degraded",
                    "Unsupported DOCX note content was ignored during import",
                );
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"r" => break,
            Event::Start(start) => {
                degraded = true;
                warnings.warn(
                    "docx_note_content_degraded",
                    "Unsupported DOCX note content was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCX_NOTES_XML)?;
            }
            Event::Empty(_) => {
                degraded = true;
                warnings.warn(
                    "docx_note_content_degraded",
                    "Unsupported DOCX note content was ignored during import",
                );
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(ParsedDocxNoteText { text, degraded })
}

fn parse_docx_comments(
    parts: &BTreeMap<String, String>,
    warnings: &mut WarningSink,
) -> Result<ImportedDocxComments, DocxError> {
    let mut imported = ImportedDocxComments::default();
    for xml in parts.values() {
        parse_docx_comments_xml(xml, &mut imported, warnings)?;
    }
    Ok(imported)
}

fn parse_docx_comments_xml(
    xml: &str,
    imported: &mut ImportedDocxComments,
    warnings: &mut WarningSink,
) -> Result<(), DocxError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCX_COMMENTS_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"comment" => {
                if imported.comments.len() >= MAX_DOCX_COMMENTS {
                    warnings.warn(
                        "docx_comments_over_limit",
                        "Excess DOCX comments were ignored during import",
                    );
                    skip_element(&mut reader, b"comment", DOCX_COMMENTS_XML)?;
                    continue;
                }
                parse_docx_comment(&mut reader, &start, imported, warnings)?;
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"comment" => {
                warnings.warn(
                    "docx_comment_ignored",
                    "Unsupported DOCX comments were ignored during import",
                );
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(())
}

fn parse_docx_comment(
    reader: &mut Reader<&[u8]>,
    start: &BytesStart<'_>,
    imported: &mut ImportedDocxComments,
    warnings: &mut WarningSink,
) -> Result<(), DocxError> {
    let Some(raw_id) = attr_value(start, b"id", DOCX_COMMENTS_XML)? else {
        warnings.warn(
            "docx_comment_ignored",
            "Unsupported DOCX comments were ignored during import",
        );
        skip_element(reader, b"comment", DOCX_COMMENTS_XML)?;
        return Ok(());
    };
    if imported.by_raw_id.contains_key(&raw_id) {
        warnings.warn(
            "docx_comment_ignored",
            "Unsupported DOCX comments were ignored during import",
        );
        skip_element(reader, b"comment", DOCX_COMMENTS_XML)?;
        return Ok(());
    }

    let author = attr_value(start, b"author", DOCX_COMMENTS_XML)?;
    let created_at = attr_value(start, b"date", DOCX_COMMENTS_XML)?
        .and_then(|value| DateTime::parse_from_rfc3339(value.trim()).ok())
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let mut paragraphs = Vec::new();

    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCX_COMMENTS_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"p" => {
                paragraphs.push(parse_docx_comment_paragraph(reader, warnings)?);
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"comment" => break,
            Event::Start(start) => {
                warnings.warn(
                    "docx_comment_content_degraded",
                    "Unsupported DOCX comment content was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCX_COMMENTS_XML)?;
            }
            Event::Empty(_) => {
                warnings.warn(
                    "docx_comment_content_degraded",
                    "Unsupported DOCX comment content was ignored during import",
                );
            }
            Event::Eof => break,
            _ => {}
        }
    }

    let body = paragraphs
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let Ok(body) = validate_comment_body(&body) else {
        warnings.warn(
            "docx_comment_ignored",
            "Unsupported DOCX comments were ignored during import",
        );
        return Ok(());
    };
    let Ok(author) = normalize_comment_author(author.as_deref()) else {
        warnings.warn(
            "docx_comment_ignored",
            "Unsupported DOCX comments were ignored during import",
        );
        return Ok(());
    };
    let local_id = next_imported_docx_comment_id(&raw_id, imported.comments.len() + 1);
    imported.by_raw_id.insert(
        raw_id,
        ImportedDocxComment {
            local_id: local_id.clone(),
        },
    );
    imported.comments.insert(
        local_id.clone(),
        CommentThread {
            id: local_id,
            author,
            body,
            created_at,
            updated_at: created_at,
            resolved: false,
        },
    );
    Ok(())
}

fn parse_docx_comment_paragraph(
    reader: &mut Reader<&[u8]>,
    warnings: &mut WarningSink,
) -> Result<String, DocxError> {
    let mut text = String::new();
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCX_COMMENTS_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"r" => {
                text.push_str(&parse_docx_comment_run(reader, warnings)?);
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"p" => break,
            Event::Start(start) => {
                let local = local_name(start.name().as_ref()).to_vec();
                if local != b"pPr" {
                    warnings.warn(
                        "docx_comment_content_degraded",
                        "Unsupported DOCX comment content was ignored during import",
                    );
                }
                skip_element(reader, &local, DOCX_COMMENTS_XML)?;
            }
            Event::Empty(start) => {
                if local_name(start.name().as_ref()) != b"pPr" {
                    warnings.warn(
                        "docx_comment_content_degraded",
                        "Unsupported DOCX comment content was ignored during import",
                    );
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(text)
}

fn parse_docx_comment_run(
    reader: &mut Reader<&[u8]>,
    warnings: &mut WarningSink,
) -> Result<String, DocxError> {
    let mut text = String::new();
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCX_COMMENTS_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"t" => {
                text.push_str(&read_text_element(reader, b"t", DOCX_COMMENTS_XML)?);
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"tab" => {
                text.push('\t');
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"br" => {
                text.push('\n');
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"r" => break,
            Event::Start(start) => {
                let local = local_name(start.name().as_ref()).to_vec();
                if local != b"rPr" {
                    warnings.warn(
                        "docx_comment_content_degraded",
                        "Unsupported DOCX comment content was ignored during import",
                    );
                }
                skip_element(reader, &local, DOCX_COMMENTS_XML)?;
            }
            Event::Empty(start) => {
                if !matches!(local_name(start.name().as_ref()), b"rPr" | b"tab" | b"br") {
                    warnings.warn(
                        "docx_comment_content_degraded",
                        "Unsupported DOCX comment content was ignored during import",
                    );
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(text)
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
    context: &DocxImportContext<'_>,
    page_region_part_xml: &BTreeMap<String, String>,
    numbering: &NumberingMap,
    warnings: &mut WarningSink,
) -> Result<ParsedDocument, DocxError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut parsed = Vec::new();
    let mut page_regions = PageRegions::default();
    let mut page_setup = None;
    let mut anchored_comment_ids = BTreeSet::new();
    let mut revision_state = RevisionImportState::default();
    let mut note_state = NoteImportState::default();
    let mut in_body = false;

    {
        let mut state = DocxBodyParseState {
            warnings,
            anchored_comment_ids: &mut anchored_comment_ids,
            revisions: &mut revision_state,
            notes: &mut note_state,
        };

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
                    let blocks = parse_paragraph(&mut reader, context, numbering, &mut state)?;
                    for block in blocks {
                        push_parsed_block(&mut parsed, block);
                    }
                }
                Event::Start(start) if in_body && local_name(start.name().as_ref()) == b"tbl" => {
                    let table = parse_table(&mut reader, context, numbering, &mut state)?;
                    push_parsed_block(
                        &mut parsed,
                        ParsedBlock {
                            block: Block::Table(table),
                            list_marker: None,
                            toc_role: None,
                            counts_for_cell_alignment: false,
                            paragraph_alignment: None,
                        },
                    );
                }
                Event::Start(start)
                    if in_body && local_name(start.name().as_ref()) == b"sectPr" =>
                {
                    let properties = parse_section_properties(&mut reader, state.warnings)?;
                    page_setup = properties.page_setup;
                    page_regions = build_page_regions(
                        &properties.page_regions,
                        context.rels,
                        page_region_part_xml,
                        state.warnings,
                    )?;
                }
                Event::Empty(_) if in_body => {
                    state.warnings.warn(
                        "docx_unsupported_body_content",
                        "Unsupported DOCX body content was ignored during import",
                    );
                }
                Event::Start(start) if in_body => {
                    state.warnings.warn(
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
    }

    if context
        .notes
        .has_unanchored(&note_state.referenced_raw_keys())
    {
        warnings.warn(
            "docx_unanchored_notes_ignored",
            "Unsupported DOCX notes without supported body references were ignored during import",
        );
    }

    Ok(ParsedDocument {
        blocks: parsed.into_iter().map(|item| item.block).collect(),
        page_setup,
        page_regions,
        anchored_comment_ids,
        notes: note_state.notes,
    })
}

fn parse_section_properties(
    reader: &mut Reader<&[u8]>,
    warnings: &mut WarningSink,
) -> Result<SectionProperties, DocxError> {
    let mut properties = SectionProperties::default();
    let mut page_setup = PageSetup::default();
    let mut saw_page_setup_tag = false;
    let mut saw_page_size = false;
    let mut saw_page_margins = false;
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"headerReference" =>
            {
                apply_page_region_reference(
                    &mut properties.page_regions,
                    PageRegionPartKind::Header,
                    &start,
                    warnings,
                )?;
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"footerReference" =>
            {
                apply_page_region_reference(
                    &mut properties.page_regions,
                    PageRegionPartKind::Footer,
                    &start,
                    warnings,
                )?;
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"titlePg" =>
            {
                properties.page_regions.different_first_page =
                    truthy_word_bool(&start, DOCUMENT_XML)?;
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"pgSz" =>
            {
                saw_page_setup_tag = true;
                if apply_docx_page_size(&start, &mut page_setup)? {
                    saw_page_size = true;
                }
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"pgMar" =>
            {
                saw_page_setup_tag = true;
                if apply_docx_page_margins(&start, &mut page_setup)? {
                    saw_page_margins = true;
                }
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
    if saw_page_size && saw_page_margins && page_setup.validate().is_ok() {
        properties.page_setup = Some(page_setup);
    } else if saw_page_setup_tag {
        warnings.warn(
            "docx_page_setup_ignored",
            "Unsupported DOCX page setup was ignored during import",
        );
    }
    Ok(properties)
}

fn apply_docx_page_size(
    start: &BytesStart<'_>,
    page_setup: &mut PageSetup,
) -> Result<bool, DocxError> {
    let Some(width_twips) = parse_docx_i32_attr(start, b"w")? else {
        return Ok(false);
    };
    let Some(height_twips) = parse_docx_i32_attr(start, b"h")? else {
        return Ok(false);
    };
    let Some(width_mm) = docx_twips_to_page_dimension_mm(width_twips) else {
        return Ok(false);
    };
    let Some(height_mm) = docx_twips_to_page_dimension_mm(height_twips) else {
        return Ok(false);
    };
    page_setup.width_mm = width_mm;
    page_setup.height_mm = height_mm;
    Ok(true)
}

fn apply_docx_page_margins(
    start: &BytesStart<'_>,
    page_setup: &mut PageSetup,
) -> Result<bool, DocxError> {
    let Some(top_twips) = parse_docx_i32_attr(start, b"top")? else {
        return Ok(false);
    };
    let Some(right_twips) = parse_docx_i32_attr(start, b"right")? else {
        return Ok(false);
    };
    let Some(bottom_twips) = parse_docx_i32_attr(start, b"bottom")? else {
        return Ok(false);
    };
    let Some(left_twips) = parse_docx_i32_attr(start, b"left")? else {
        return Ok(false);
    };
    let Some(top_mm) = docx_twips_to_page_margin_mm(top_twips) else {
        return Ok(false);
    };
    let Some(right_mm) = docx_twips_to_page_margin_mm(right_twips) else {
        return Ok(false);
    };
    let Some(bottom_mm) = docx_twips_to_page_margin_mm(bottom_twips) else {
        return Ok(false);
    };
    let Some(left_mm) = docx_twips_to_page_margin_mm(left_twips) else {
        return Ok(false);
    };
    page_setup.margin_top_mm = top_mm;
    page_setup.margin_right_mm = right_mm;
    page_setup.margin_bottom_mm = bottom_mm;
    page_setup.margin_left_mm = left_mm;
    Ok(true)
}

fn docx_twips_to_page_dimension_mm(twips: i32) -> Option<u16> {
    let millimeters = docx_twips_to_bounded_u16_mm(twips, 500)?;
    if millimeters >= 50 {
        Some(millimeters)
    } else {
        None
    }
}

fn docx_twips_to_page_margin_mm(twips: i32) -> Option<u16> {
    docx_twips_to_bounded_u16_mm(twips, 100)
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
    context: &DocxImportContext<'_>,
    numbering: &NumberingMap,
    state: &mut DocxBodyParseState<'_>,
) -> Result<Table, DocxError> {
    let mut rows = Vec::new();
    let mut grid_widths = None;
    let mut seen_grid = false;
    let mut unsupported_width_shape = false;
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"tblGrid" => {
                if !seen_grid {
                    seen_grid = true;
                    grid_widths = parse_table_grid_widths(reader)?;
                } else {
                    unsupported_width_shape = true;
                    grid_widths = None;
                    skip_element(reader, b"tblGrid", DOCUMENT_XML)?;
                }
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"tblGrid" => {
                if !seen_grid {
                    seen_grid = true;
                    grid_widths = None;
                } else {
                    unsupported_width_shape = true;
                    grid_widths = None;
                }
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"tr" => {
                rows.push(parse_table_row(
                    reader,
                    context,
                    numbering,
                    state,
                    &mut unsupported_width_shape,
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
    let mut table = Table {
        column_widths: Vec::new(),
        rows,
    };
    if !unsupported_width_shape {
        if let Some(column_count) = table.editable_column_count() {
            if let Some(widths) = grid_widths
                .as_deref()
                .and_then(|widths| normalize_docx_table_grid_widths(widths, column_count))
            {
                table.column_widths = widths;
            }
        }
    }
    Ok(table)
}

fn parse_table_grid_widths(reader: &mut Reader<&[u8]>) -> Result<Option<Vec<u64>>, DocxError> {
    let mut widths = Vec::new();
    let mut valid = true;
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Empty(start) if local_name(start.name().as_ref()) == b"gridCol" => {
                valid &= push_docx_grid_column_width(&start, &mut widths)?;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"gridCol" => {
                valid &= push_docx_grid_column_width(&start, &mut widths)?;
                skip_element(reader, b"gridCol", DOCUMENT_XML)?;
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"tblGrid" => break,
            Event::Start(start) => {
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if valid && !widths.is_empty() && widths.len() <= MAX_TABLE_WIDTH_COLUMNS {
        Ok(Some(widths))
    } else {
        Ok(None)
    }
}

fn push_docx_grid_column_width(
    start: &BytesStart<'_>,
    widths: &mut Vec<u64>,
) -> Result<bool, DocxError> {
    let Some(value) = attr_value(start, b"w", DOCUMENT_XML)? else {
        return Ok(false);
    };
    let Ok(width) = value.parse::<u64>() else {
        return Ok(false);
    };
    if width == 0 {
        return Ok(false);
    }
    widths.push(width);
    Ok(widths.len() <= MAX_TABLE_WIDTH_COLUMNS)
}

fn normalize_docx_table_grid_widths(widths: &[u64], column_count: usize) -> Option<Vec<u16>> {
    if widths.len() != column_count {
        return None;
    }
    let total = widths
        .iter()
        .try_fold(0_u128, |total, width| total.checked_add(u128::from(*width)))?;
    if total == 0 {
        return None;
    }

    let mut normalized = Vec::with_capacity(widths.len());
    let mut remainders = Vec::with_capacity(widths.len());
    let mut normalized_total = 0_u16;
    for (index, width) in widths.iter().enumerate() {
        let scaled = u128::from(*width).checked_mul(1000)?;
        let value = (scaled / total) as u16;
        normalized_total = normalized_total.checked_add(value)?;
        normalized.push(value);
        remainders.push((index, scaled % total));
    }

    let mut remaining = 1000_u16.checked_sub(normalized_total)?;
    remainders.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    for (index, _) in remainders {
        if remaining == 0 {
            break;
        }
        normalized[index] = normalized[index].checked_add(1)?;
        remaining -= 1;
    }

    sanitize_table_column_widths(&normalized, column_count)
}

fn docx_paragraph_alignment(value: String) -> Option<ParagraphAlignment> {
    match value.as_str() {
        "left" | "start" => Some(ParagraphAlignment::Left),
        "center" => Some(ParagraphAlignment::Center),
        "right" | "end" => Some(ParagraphAlignment::Right),
        "both" | "distribute" => Some(ParagraphAlignment::Justify),
        _ => None,
    }
}

fn docx_twips_to_mm(twips: i32) -> Option<i16> {
    let millimeters = (f64::from(twips) * 25.4 / 1440.0).round();
    if millimeters < f64::from(i16::MIN) || millimeters > f64::from(i16::MAX) {
        return None;
    }
    Some(millimeters as i16)
}

fn docx_twips_to_bounded_u16_mm(twips: i32, max_mm: u16) -> Option<u16> {
    if twips < 0 {
        return None;
    }
    let millimeters = docx_twips_to_mm(twips)?;
    if millimeters < 0 || millimeters > max_mm as i16 {
        return None;
    }
    Some(millimeters as u16)
}

fn docx_twips_to_bounded_i16_mm(twips: i32, max_abs_mm: i16) -> Option<i16> {
    let millimeters = docx_twips_to_mm(twips)?;
    if i32::from(millimeters).abs() > i32::from(max_abs_mm) {
        return None;
    }
    Some(millimeters)
}

fn mm_to_docx_twips(mm: u16) -> u32 {
    (f64::from(mm) * 1440.0 / 25.4).round() as u32
}

fn signed_mm_to_docx_twips(mm: i16) -> u32 {
    (f64::from(mm.unsigned_abs()) * 1440.0 / 25.4).round() as u32
}

fn docx_line_spacing_to_per_mille(start: &BytesStart<'_>) -> Result<Option<u16>, DocxError> {
    let Some(line) = attr_value(start, b"line", DOCUMENT_XML)? else {
        return Ok(None);
    };
    if !matches!(
        attr_value(start, b"lineRule", DOCUMENT_XML)?.as_deref(),
        None | Some("auto")
    ) {
        return Ok(None);
    }
    let Ok(line) = line.parse::<u32>() else {
        return Ok(None);
    };
    let per_mille = ((u64::from(line) * 1000) + (u64::from(DOCX_LINE_SPACING_BASE) / 2))
        / u64::from(DOCX_LINE_SPACING_BASE);
    if !(u64::from(MIN_DOCX_LINE_SPACING_PER_MILLE)..=u64::from(MAX_DOCX_LINE_SPACING_PER_MILLE))
        .contains(&per_mille)
    {
        return Ok(None);
    }
    Ok(Some(per_mille as u16))
}

fn per_mille_to_docx_line_spacing(per_mille: u16) -> u32 {
    ((u32::from(per_mille) * DOCX_LINE_SPACING_BASE) + 500) / 1000
}

fn docx_run_font_size_pt(start: &BytesStart<'_>, name: &str) -> Result<Option<u16>, DocxError> {
    let Some(value) = attr_value(start, b"val", name)? else {
        return Ok(None);
    };
    let Ok(half_points) = value.parse::<u16>() else {
        return Ok(None);
    };
    if half_points == 0 || half_points % 2 != 0 {
        return Ok(None);
    }
    let points = half_points / 2;
    if SUPPORTED_DOCX_INLINE_FONT_SIZES_PT.contains(&points) {
        Ok(Some(points))
    } else {
        Ok(None)
    }
}

fn docx_run_text_color(start: &BytesStart<'_>, name: &str) -> Result<Option<String>, DocxError> {
    if attr_value(start, b"themeColor", name)?.is_some() {
        return Ok(None);
    }
    let Some(value) = attr_value(start, b"val", name)? else {
        return Ok(None);
    };
    Ok(docx_hex_color(&value))
}

fn docx_hex_color(value: &str) -> Option<String> {
    if value.len() == 6 && value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Some(format!("#{}", value.to_ascii_lowercase()))
    } else {
        None
    }
}

fn docx_highlight_color(value: &str) -> Option<&'static str> {
    match value {
        "yellow" => Some("#fff3bf"),
        "cyan" => Some("#dbeafe"),
        "green" => Some("#dcfce7"),
        "lightGray" => Some("#f1f5f9"),
        _ => None,
    }
}

fn docx_highlight_value(color: &str) -> Option<&'static str> {
    match color.trim().to_ascii_lowercase().as_str() {
        "#fff3bf" => Some("yellow"),
        "#dbeafe" => Some("cyan"),
        "#dcfce7" => Some("green"),
        "#f1f5f9" => Some("lightGray"),
        _ => None,
    }
}

fn docx_text_color_value(color: &str) -> Option<String> {
    docx_hex_color(color.strip_prefix('#')?).map(|value| value[1..].to_ascii_uppercase())
}

fn parse_docx_i32_attr(start: &BytesStart<'_>, attr: &[u8]) -> Result<Option<i32>, DocxError> {
    let Some(value) = attr_value(start, attr, DOCUMENT_XML)? else {
        return Ok(None);
    };
    Ok(value.parse::<i32>().ok())
}

fn parse_table_row(
    reader: &mut Reader<&[u8]>,
    context: &DocxImportContext<'_>,
    numbering: &NumberingMap,
    state: &mut DocxBodyParseState<'_>,
    unsupported_width_shape: &mut bool,
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
                    context,
                    numbering,
                    state,
                    unsupported_width_shape,
                )?);
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"trPr" => {
                if skip_table_row_properties(reader, state.warnings)? {
                    *unsupported_width_shape = true;
                }
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
    context: &DocxImportContext<'_>,
    numbering: &NumberingMap,
    state: &mut DocxBodyParseState<'_>,
    unsupported_width_shape: &mut bool,
) -> Result<TableCell, DocxError> {
    let mut parsed = Vec::new();
    let mut presentation = TableCellPresentation::default();
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"p" => {
                let blocks = parse_paragraph(reader, context, numbering, state)?;
                for block in blocks {
                    push_parsed_block(&mut parsed, block);
                }
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"tbl" => {
                *unsupported_width_shape = true;
                state.warnings.warn(
                    "docx_nested_table_degraded",
                    "Nested DOCX tables were imported as plain visible text",
                );
                let table = parse_table(reader, context, numbering, state)?;
                push_parsed_block(
                    &mut parsed,
                    ParsedBlock {
                        block: table_to_paragraph_block(&table),
                        list_marker: None,
                        toc_role: None,
                        counts_for_cell_alignment: false,
                        paragraph_alignment: None,
                    },
                );
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"tcPr" => {
                let properties = parse_table_cell_properties(reader, state.warnings)?;
                if properties.unsupported_width_shape {
                    *unsupported_width_shape = true;
                }
                presentation = properties.presentation;
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"tc" => break,
            Event::Empty(_) => {
                *unsupported_width_shape = true;
                state.warnings.warn(
                    "docx_unsupported_table_content",
                    "Unsupported DOCX table content was ignored during import",
                );
            }
            Event::Start(start) => {
                *unsupported_width_shape = true;
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }

    if presentation.text_alignment.is_none() {
        presentation.text_alignment = table_cell_text_alignment_from_parsed_blocks(&mut parsed);
    }
    let blocks = parsed
        .into_iter()
        .map(|item| item.block)
        .collect::<Vec<_>>();
    let blocks = if blocks.is_empty() {
        vec![empty_paragraph_block()]
    } else {
        blocks
    };
    Ok(TableCell {
        presentation,
        blocks,
    })
}

fn table_cell_text_alignment_from_parsed_blocks(
    parsed_blocks: &mut [ParsedBlock],
) -> Option<ParagraphAlignment> {
    let mut summary = TableCellAlignmentSummary::default();
    collect_parsed_table_cell_alignments(parsed_blocks, &mut summary);
    if summary.has_unaligned_paragraph || summary.alignments.len() != 1 {
        return None;
    }
    clear_parsed_table_cell_alignments(parsed_blocks);
    summary.alignments.first().copied()
}

#[derive(Debug, Default)]
struct TableCellAlignmentSummary {
    alignments: Vec<ParagraphAlignment>,
    has_unaligned_paragraph: bool,
}

impl TableCellAlignmentSummary {
    fn add(&mut self, alignment: Option<ParagraphAlignment>) {
        if let Some(alignment) = alignment {
            if !self.alignments.contains(&alignment) {
                self.alignments.push(alignment);
            }
        } else {
            self.has_unaligned_paragraph = true;
        }
    }
}

fn collect_parsed_table_cell_alignments(
    parsed_blocks: &[ParsedBlock],
    summary: &mut TableCellAlignmentSummary,
) {
    for parsed in parsed_blocks {
        if parsed.counts_for_cell_alignment {
            summary.add(parsed.paragraph_alignment);
        } else {
            collect_table_cell_block_alignments(std::slice::from_ref(&parsed.block), summary);
        }
    }
}

fn collect_table_cell_block_alignments(blocks: &[Block], summary: &mut TableCellAlignmentSummary) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                summary.add(paragraph.format.alignment);
            }
            Block::List(list) => {
                for item in &list.items {
                    collect_table_cell_block_alignments(&item.blocks, summary);
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_table_cell_block_alignments(&cell.blocks, summary);
                    }
                }
            }
            Block::Heading(_) | Block::TableOfContents(_) | Block::Image(_) | Block::PageBreak => {}
        }
    }
}

fn clear_parsed_table_cell_alignments(parsed_blocks: &mut [ParsedBlock]) {
    for parsed in parsed_blocks {
        if parsed.counts_for_cell_alignment {
            if let Block::Paragraph(paragraph) = &mut parsed.block {
                paragraph.format.alignment = None;
            }
        } else {
            clear_table_cell_block_alignments(std::slice::from_mut(&mut parsed.block));
        }
    }
}

fn clear_table_cell_block_alignments(blocks: &mut [Block]) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => paragraph.format.alignment = None,
            Block::List(list) => {
                for item in &mut list.items {
                    clear_table_cell_block_alignments(&mut item.blocks);
                }
            }
            Block::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        clear_table_cell_block_alignments(&mut cell.blocks);
                    }
                }
            }
            Block::Heading(_) | Block::TableOfContents(_) | Block::Image(_) | Block::PageBreak => {}
        }
    }
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
    context: &DocxImportContext<'_>,
    numbering: &NumberingMap,
    state: &mut DocxBodyParseState<'_>,
) -> Result<Vec<ParsedBlock>, DocxError> {
    let mut properties = ParagraphProperties::default();
    let mut content = Vec::new();
    let mut comment_state = CommentImportState::default();

    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Start(start) if local_name(start.name().as_ref()) == b"pPr" => {
                properties = parse_paragraph_properties(reader, numbering, state.warnings)?;
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"bookmarkStart" => {
                apply_paragraph_bookmark(&mut properties, &start, state.warnings)?;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"bookmarkStart" => {
                apply_paragraph_bookmark(&mut properties, &start, state.warnings)?;
                skip_element(reader, b"bookmarkStart", DOCUMENT_XML)?;
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"bookmarkEnd" => {
                if attr_value(&start, b"id", DOCUMENT_XML)?.is_none() {
                    state.warnings.warn(
                        "docx_bookmark_ignored",
                        "Unsupported DOCX bookmarks were ignored during import",
                    );
                }
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"bookmarkEnd" => {
                if attr_value(&start, b"id", DOCUMENT_XML)?.is_none() {
                    state.warnings.warn(
                        "docx_bookmark_ignored",
                        "Unsupported DOCX bookmarks were ignored during import",
                    );
                }
                skip_element(reader, b"bookmarkEnd", DOCUMENT_XML)?;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"r" => {
                let run = parse_paragraph_run(
                    reader,
                    None,
                    context,
                    &mut comment_state,
                    state,
                    DOCUMENT_XML,
                    None,
                )?;
                append_paragraph_content(&mut content, run);
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"hyperlink" => {
                let link = hyperlink_ref(&start, context.rels, state.warnings)?;
                let run = parse_hyperlink(
                    reader,
                    link,
                    context,
                    &mut comment_state,
                    state,
                    DOCUMENT_XML,
                    None,
                )?;
                append_paragraph_content(&mut content, run);
            }
            Event::Start(start)
                if docx_tracked_change_kind(local_name(start.name().as_ref())).is_some() =>
            {
                let kind = docx_tracked_change_kind(local_name(start.name().as_ref()))
                    .expect("revision kind checked above");
                let run = parse_revision_container(
                    reader,
                    &start,
                    kind,
                    context,
                    &mut comment_state,
                    state,
                    DOCUMENT_XML,
                )?;
                append_paragraph_content(&mut content, run);
            }
            Event::Empty(start)
                if docx_tracked_change_kind(local_name(start.name().as_ref())).is_some() =>
            {
                state.warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
                );
            }
            Event::Start(start)
                if is_unsupported_docx_revision_markup(local_name(start.name().as_ref())) =>
            {
                state.warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                let text = read_visible_text_fallback(reader, &end, DOCUMENT_XML)?;
                push_plain_visible_text(&mut content, text);
            }
            Event::Empty(start)
                if is_unsupported_docx_revision_markup(local_name(start.name().as_ref())) =>
            {
                state.warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
                );
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"commentRangeStart" =>
            {
                comment_state.start_range(&start, context.comments, state.warnings)?;
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"commentRangeEnd" =>
            {
                comment_state.end_range(&start, state.warnings)?;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"fldSimple" => {
                let field = page_field_from_instruction(
                    attr_value(&start, b"instr", DOCUMENT_XML)?.as_deref(),
                );
                let run = parse_simple_field(reader, field, state.warnings, DOCUMENT_XML)?;
                append_inline_content(&mut content, run);
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"fldSimple" => {
                if let Some(field) = page_field_from_instruction(
                    attr_value(&start, b"instr", DOCUMENT_XML)?.as_deref(),
                ) {
                    content.push(ParagraphContent::Inline(Box::new(Inline::field(field))));
                } else {
                    state.warnings.warn(
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
                state.warnings.warn(
                    "docx_media_ignored",
                    "Unsupported DOCX media content was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"p" => break,
            Event::Empty(_) => {
                state.warnings.warn(
                    "docx_unsupported_paragraph_content",
                    "Unsupported DOCX paragraph content was ignored during import",
                );
            }
            Event::Start(start) => {
                state.warnings.warn(
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

    comment_state.finish_paragraph(state.anchored_comment_ids, state.warnings);
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
                    if let Some(role) = toc_role_from_style(&style) {
                        properties.toc_role = Some(role);
                    } else {
                        properties.heading_level = heading_level_from_style(&style);
                    }
                }
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"numPr" => {
                properties.list_marker = parse_num_properties(reader, numbering, warnings)?;
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"numPr" => {}
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"jc" =>
            {
                properties.format.alignment =
                    attr_value(&start, b"val", DOCUMENT_XML)?.and_then(docx_paragraph_alignment);
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"spacing" => {
                apply_docx_paragraph_spacing(&start, &mut properties.format)?;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"spacing" => {
                apply_docx_paragraph_spacing(&start, &mut properties.format)?;
                skip_element(reader, b"spacing", DOCUMENT_XML)?;
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"ind" => {
                apply_docx_paragraph_indent(&start, &mut properties.format)?;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"ind" => {
                apply_docx_paragraph_indent(&start, &mut properties.format)?;
                skip_element(reader, b"ind", DOCUMENT_XML)?;
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"pPr" => break,
            Event::Start(start)
                if is_any_docx_revision_markup(local_name(start.name().as_ref())) =>
            {
                warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Empty(start)
                if is_any_docx_revision_markup(local_name(start.name().as_ref())) =>
            {
                warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
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

    Ok(properties)
}

fn apply_docx_paragraph_spacing(
    start: &BytesStart<'_>,
    format: &mut ParagraphFormat,
) -> Result<(), DocxError> {
    if let Some(line_spacing) = docx_line_spacing_to_per_mille(start)? {
        format.line_spacing_per_mille = Some(line_spacing);
    }
    if let Some(twips) = parse_docx_i32_attr(start, b"before")? {
        if let Some(mm) = docx_twips_to_bounded_u16_mm(twips, MAX_DOCX_PARAGRAPH_SPACING_MM) {
            format.spacing_before_mm = Some(mm);
        }
    }
    if let Some(twips) = parse_docx_i32_attr(start, b"after")? {
        if let Some(mm) = docx_twips_to_bounded_u16_mm(twips, MAX_DOCX_PARAGRAPH_SPACING_MM) {
            format.spacing_after_mm = Some(mm);
        }
    }
    Ok(())
}

fn apply_docx_paragraph_indent(
    start: &BytesStart<'_>,
    format: &mut ParagraphFormat,
) -> Result<(), DocxError> {
    if let Some(twips) =
        parse_docx_i32_attr(start, b"left")?.or(parse_docx_i32_attr(start, b"start")?)
    {
        if let Some(mm) = docx_twips_to_bounded_u16_mm(twips, MAX_DOCX_PARAGRAPH_INDENT_MM) {
            format.indent_start_mm = Some(mm);
        }
    }
    if let Some(twips) =
        parse_docx_i32_attr(start, b"right")?.or(parse_docx_i32_attr(start, b"end")?)
    {
        if let Some(mm) = docx_twips_to_bounded_u16_mm(twips, MAX_DOCX_PARAGRAPH_INDENT_MM) {
            format.indent_end_mm = Some(mm);
        }
    }
    if let Some(twips) = parse_docx_i32_attr(start, b"firstLine")? {
        if let Some(mm) = docx_twips_to_bounded_i16_mm(twips, MAX_DOCX_FIRST_LINE_INDENT_MM) {
            format.first_line_indent_mm = Some(mm);
        }
    } else if let Some(twips) = parse_docx_i32_attr(start, b"hanging")? {
        if let Some(mm) = docx_twips_to_bounded_i16_mm(twips, MAX_DOCX_FIRST_LINE_INDENT_MM) {
            format.first_line_indent_mm = Some(-mm);
        }
    }
    Ok(())
}

fn apply_paragraph_bookmark(
    properties: &mut ParagraphProperties,
    start: &BytesStart<'_>,
    warnings: &mut WarningSink,
) -> Result<(), DocxError> {
    let Some(name) = attr_value(start, b"name", DOCUMENT_XML)? else {
        warnings.warn(
            "docx_bookmark_ignored",
            "Unsupported DOCX bookmarks were ignored during import",
        );
        return Ok(());
    };
    let Some(bookmark_id) = sanitize_bookmark_id(&name) else {
        warnings.warn(
            "docx_bookmark_ignored",
            "Unsupported DOCX bookmarks were ignored during import",
        );
        return Ok(());
    };
    if properties.bookmark_id.is_none() {
        properties.bookmark_id = Some(bookmark_id);
    }
    Ok(())
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
            Event::Start(start)
                if is_any_docx_revision_markup(local_name(start.name().as_ref())) =>
            {
                warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Empty(start)
                if is_any_docx_revision_markup(local_name(start.name().as_ref())) =>
            {
                warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
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
    context: &DocxImportContext<'_>,
    comment_state: &mut CommentImportState,
    state: &mut DocxBodyParseState<'_>,
    name: &str,
    tracked_change: Option<&TrackedChange>,
) -> Result<Vec<ParagraphContent>, DocxError> {
    let mut content = Vec::new();
    loop {
        match reader.read_event().map_err(|err| xml_error(name, err))? {
            Event::Start(start) if local_name(start.name().as_ref()) == b"r" => {
                let run = parse_paragraph_run(
                    reader,
                    link.href.clone(),
                    context,
                    comment_state,
                    state,
                    name,
                    tracked_change,
                )?;
                append_paragraph_content(&mut content, run);
            }
            Event::Start(start)
                if docx_tracked_change_kind(local_name(start.name().as_ref())).is_some() =>
            {
                state.warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                let text = read_visible_text_fallback(reader, &end, name)?;
                push_plain_visible_text(&mut content, text);
            }
            Event::Start(start)
                if is_unsupported_docx_revision_markup(local_name(start.name().as_ref())) =>
            {
                state.warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                let text = read_visible_text_fallback(reader, &end, name)?;
                push_plain_visible_text(&mut content, text);
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
    context: &DocxImportContext<'_>,
    comment_state: &mut CommentImportState,
    state: &mut DocxBodyParseState<'_>,
    name: &str,
    tracked_change: Option<&TrackedChange>,
) -> Result<Vec<ParagraphContent>, DocxError> {
    let mut properties = RunProperties::default();
    let mut text = String::new();
    let mut content = Vec::new();

    loop {
        match reader.read_event().map_err(|err| xml_error(name, err))? {
            Event::Start(start) if local_name(start.name().as_ref()) == b"rPr" => {
                properties = parse_run_properties(reader, name, state.warnings)?;
            }
            Event::Start(start)
                if matches!(local_name(start.name().as_ref()), b"t" | b"delText") =>
            {
                let end = local_name(start.name().as_ref()).to_vec();
                text.push_str(&read_text_element(reader, &end, name)?);
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"tab" => {
                text.push('\t');
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"br" => {
                text.push('\n');
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"drawing" => {
                flush_run_text_content(
                    &mut content,
                    &mut text,
                    &properties,
                    link.clone(),
                    Some(comment_state),
                    tracked_change,
                );
                if let Some(image) =
                    parse_drawing(reader, context.rels, context.images, state.warnings, name)?
                {
                    content.push(ParagraphContent::Image(image));
                }
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"drawing" => {
                state.warnings.warn(
                    "docx_image_reference_ignored",
                    "Unsupported DOCX image references were ignored during import",
                );
            }
            Event::Empty(start)
                if matches!(local_name(start.name().as_ref()), b"object" | b"pict") =>
            {
                state.warnings.warn(
                    "docx_media_ignored",
                    "Unsupported DOCX media content was ignored during import",
                );
            }
            Event::Start(start)
                if matches!(local_name(start.name().as_ref()), b"object" | b"pict") =>
            {
                state.warnings.warn(
                    "docx_media_ignored",
                    "Unsupported DOCX media content was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, name)?;
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"commentReference" => {
                comment_state.reference_marker(&start, context.comments, state.warnings)?;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"commentReference" => {
                comment_state.reference_marker(&start, context.comments, state.warnings)?;
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, name)?;
            }
            Event::Empty(start)
                if docx_note_reference_kind(local_name(start.name().as_ref())).is_some() =>
            {
                flush_run_text_content(
                    &mut content,
                    &mut text,
                    &properties,
                    link.clone(),
                    Some(comment_state),
                    tracked_change,
                );
                let kind = docx_note_reference_kind(local_name(start.name().as_ref()))
                    .expect("note reference kind checked above");
                push_docx_note_reference_content(
                    &mut content,
                    &properties,
                    context.notes,
                    state,
                    &start,
                    kind,
                    tracked_change.is_some(),
                )?;
            }
            Event::Start(start)
                if docx_note_reference_kind(local_name(start.name().as_ref())).is_some() =>
            {
                flush_run_text_content(
                    &mut content,
                    &mut text,
                    &properties,
                    link.clone(),
                    Some(comment_state),
                    tracked_change,
                );
                let kind = docx_note_reference_kind(local_name(start.name().as_ref()))
                    .expect("note reference kind checked above");
                push_docx_note_reference_content(
                    &mut content,
                    &properties,
                    context.notes,
                    state,
                    &start,
                    kind,
                    tracked_change.is_some(),
                )?;
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, name)?;
            }
            Event::Empty(start)
                if matches!(local_name(start.name().as_ref()), b"fldChar" | b"instrText") =>
            {
                state.warnings.warn(
                    "docx_inline_metadata_ignored",
                    "Unsupported DOCX inline metadata was ignored during import",
                );
            }
            Event::Start(start)
                if matches!(local_name(start.name().as_ref()), b"fldChar" | b"instrText") =>
            {
                state.warnings.warn(
                    "docx_inline_metadata_ignored",
                    "Unsupported DOCX inline metadata was ignored during import",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, name)?;
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"r" => break,
            Event::Empty(_) => {
                state.warnings.warn(
                    "docx_unsupported_run_content",
                    "Unsupported DOCX run content was ignored during import",
                );
            }
            Event::Start(start) => {
                state.warnings.warn(
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

    flush_run_text_content(
        &mut content,
        &mut text,
        &properties,
        link,
        Some(comment_state),
        tracked_change,
    );
    Ok(content)
}

fn push_docx_note_reference_content(
    content: &mut Vec<ParagraphContent>,
    properties: &RunProperties,
    imported_notes: &ImportedDocxNotes,
    state: &mut DocxBodyParseState<'_>,
    start: &BytesStart<'_>,
    kind: NoteKind,
    hidden_context: bool,
) -> Result<(), DocxError> {
    if let Some(reference) = state.notes.reference(
        start,
        kind,
        imported_notes,
        state.warnings,
        DOCUMENT_XML,
        hidden_context,
    )? {
        let mut inline = Inline::note_reference(reference);
        inline.marks = properties.marks();
        inline.style = properties.style.clone();
        content.push(ParagraphContent::Inline(Box::new(inline)));
    } else {
        let mut inline = Inline::text(docx_note_reference_fallback_text(kind));
        inline.marks = properties.marks();
        inline.style = properties.style.clone();
        content.push(ParagraphContent::Inline(Box::new(inline)));
    }
    Ok(())
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

fn parse_revision_container(
    reader: &mut Reader<&[u8]>,
    start: &BytesStart<'_>,
    kind: TrackedChangeKind,
    context: &DocxImportContext<'_>,
    comment_state: &mut CommentImportState,
    state: &mut DocxBodyParseState<'_>,
    name: &str,
) -> Result<Vec<ParagraphContent>, DocxError> {
    let end_name = local_name(start.name().as_ref()).to_vec();
    let tracked_change = state
        .revisions
        .tracked_change(start, kind, state.warnings, name)?;
    let mut content = Vec::new();

    loop {
        match reader.read_event().map_err(|err| xml_error(name, err))? {
            Event::Start(start) if local_name(start.name().as_ref()) == b"r" => {
                let run = parse_paragraph_run(
                    reader,
                    None,
                    context,
                    comment_state,
                    state,
                    name,
                    tracked_change.as_ref(),
                )?;
                append_paragraph_content(&mut content, run);
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"hyperlink" => {
                let link = hyperlink_ref(&start, context.rels, state.warnings)?;
                let run = parse_hyperlink(
                    reader,
                    link,
                    context,
                    comment_state,
                    state,
                    name,
                    tracked_change.as_ref(),
                )?;
                append_paragraph_content(&mut content, run);
            }
            Event::Start(start)
                if docx_tracked_change_kind(local_name(start.name().as_ref())).is_some()
                    || is_unsupported_docx_revision_markup(local_name(start.name().as_ref())) =>
            {
                state.warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                let text = read_visible_text_fallback(reader, &end, name)?;
                push_plain_visible_text(&mut content, text);
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"fldSimple" => {
                state.warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                let text = read_visible_text_fallback(reader, &end, name)?;
                push_plain_visible_text(&mut content, text);
            }
            Event::Empty(start)
                if docx_tracked_change_kind(local_name(start.name().as_ref())).is_some()
                    || is_unsupported_docx_revision_markup(local_name(start.name().as_ref()))
                    || local_name(start.name().as_ref()) == b"fldSimple" =>
            {
                state.warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
                );
            }
            Event::Start(start)
                if matches!(
                    local_name(start.name().as_ref()),
                    b"drawing" | b"object" | b"pict"
                ) =>
            {
                state.warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, name)?;
            }
            Event::Empty(start)
                if matches!(
                    local_name(start.name().as_ref()),
                    b"drawing" | b"object" | b"pict"
                ) =>
            {
                state.warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
                );
            }
            Event::End(end) if local_name(end.name().as_ref()) == end_name.as_slice() => break,
            Event::Start(start) => {
                state.warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                let text = read_visible_text_fallback(reader, &end, name)?;
                push_plain_visible_text(&mut content, text);
            }
            Event::Empty(_) => {
                state.warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
                );
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(content)
}

fn flush_run_text_content(
    content: &mut Vec<ParagraphContent>,
    text: &mut String,
    properties: &RunProperties,
    link: Option<String>,
    comment_state: Option<&mut CommentImportState>,
    tracked_change: Option<&TrackedChange>,
) {
    if text.is_empty() {
        return;
    }
    let comment_ids = comment_state
        .map(|state| {
            state.mark_visible_text();
            state.active_local_ids()
        })
        .unwrap_or_default();
    content.push(ParagraphContent::Inline(Box::new(Inline {
        text: std::mem::take(text),
        marks: properties.marks(),
        link,
        comment_ids,
        style: properties.style.clone(),
        field: None,
        note_reference: None,
        tracked_change: tracked_change.cloned(),
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
                properties = parse_run_properties(reader, name, warnings)?;
            }
            Event::Start(start)
                if matches!(local_name(start.name().as_ref()), b"t" | b"delText") =>
            {
                let end = local_name(start.name().as_ref()).to_vec();
                text.push_str(&read_text_element(reader, &end, name)?);
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
        style: properties.style,
        field: None,
        note_reference: None,
        tracked_change: None,
    }])
}

fn parse_run_properties(
    reader: &mut Reader<&[u8]>,
    name: &str,
    warnings: &mut WarningSink,
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
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"strike" =>
            {
                properties.strike = truthy_word_bool(&start, name)?;
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"dstrike" =>
            {
                properties.double_strike = truthy_word_bool(&start, name)?;
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"vertAlign" =>
            {
                match attr_value(&start, b"val", name)?.as_deref() {
                    Some("superscript") => {
                        properties.superscript = true;
                        properties.subscript = false;
                    }
                    Some("subscript") => {
                        properties.subscript = true;
                        properties.superscript = false;
                    }
                    _ => {}
                }
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"sz" =>
            {
                if let Some(font_size) = docx_run_font_size_pt(&start, name)? {
                    properties.style.font_size_pt = Some(font_size);
                }
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"color" =>
            {
                if let Some(color) = docx_run_text_color(&start, name)? {
                    properties.style.text_color = Some(color);
                }
            }
            Event::Empty(start) | Event::Start(start)
                if local_name(start.name().as_ref()) == b"highlight" =>
            {
                if let Some(highlight) = attr_value(&start, b"val", name)?
                    .as_deref()
                    .and_then(docx_highlight_color)
                {
                    properties.style.highlight_color = Some(highlight.to_string());
                }
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"rPr" => break,
            Event::Start(start)
                if is_any_docx_revision_markup(local_name(start.name().as_ref())) =>
            {
                warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
                );
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, name)?;
            }
            Event::Empty(start)
                if is_any_docx_revision_markup(local_name(start.name().as_ref())) =>
            {
                warnings.warn(
                    "docx_revision_markup_degraded",
                    "Unsupported DOCX tracked-change markup was imported as visible text when available",
                );
            }
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
        let inlines = parse_simple_field_inlines(reader, warnings, name)?;
        let mut inline = Inline::field(field);
        if let Some((marks, style)) = common_inline_format(&inlines) {
            inline.marks = marks;
            inline.style = style;
        }
        return Ok(vec![inline]);
    }

    warnings.warn(
        "docx_field_degraded",
        "Unsupported DOCX fields were imported as visible text when available",
    );
    parse_simple_field_inlines(reader, warnings, name)
}

fn parse_simple_field_inlines(
    reader: &mut Reader<&[u8]>,
    warnings: &mut WarningSink,
    name: &str,
) -> Result<Vec<Inline>, DocxError> {
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

fn common_inline_format(inlines: &[Inline]) -> Option<(Vec<InlineMark>, InlineStyle)> {
    let mut formatted = inlines.iter().filter(|inline| !inline.text.is_empty());
    let first = formatted.next()?;
    if formatted.all(|inline| inline.marks == first.marks && inline.style == first.style) {
        Some((first.marks.clone(), first.style.clone()))
    } else {
        None
    }
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
    if parsed.list_marker.is_none() {
        if let Some(role) = parsed.toc_role {
            if push_toc_parsed_block(blocks, &parsed.block, role) {
                return;
            }
        }
    }

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
        toc_role: None,
        counts_for_cell_alignment: false,
        paragraph_alignment: None,
    });
}

fn push_toc_parsed_block(
    blocks: &mut Vec<ParsedBlock>,
    block: &Block,
    role: TocParagraphRole,
) -> bool {
    match role {
        TocParagraphRole::Title => {
            let Some(title) = toc_paragraph_text(block) else {
                return false;
            };
            blocks.push(ParsedBlock {
                block: Block::TableOfContents(TableOfContents {
                    title,
                    entries: Vec::new(),
                }),
                list_marker: None,
                toc_role: None,
                counts_for_cell_alignment: false,
                paragraph_alignment: None,
            });
            true
        }
        TocParagraphRole::Entry(level) => {
            let Some(entry) = toc_entry_from_block(block, level) else {
                return false;
            };
            if let Some(ParsedBlock {
                block: Block::TableOfContents(table_of_contents),
                ..
            }) = blocks.last_mut()
            {
                table_of_contents.entries.push(entry);
                return true;
            }
            blocks.push(ParsedBlock {
                block: Block::TableOfContents(TableOfContents::new(vec![entry])),
                list_marker: None,
                toc_role: None,
                counts_for_cell_alignment: false,
                paragraph_alignment: None,
            });
            true
        }
    }
}

fn toc_paragraph_text(block: &Block) -> Option<String> {
    let Block::Paragraph(paragraph) = block else {
        return None;
    };
    Some(
        inline_text(&paragraph.inlines)
            .trim()
            .chars()
            .take(512)
            .collect(),
    )
}

fn toc_entry_from_block(block: &Block, level: u8) -> Option<TableOfContentsEntry> {
    let Block::Paragraph(paragraph) = block else {
        return None;
    };
    let mut target = None;
    for inline in &paragraph.inlines {
        if inline.text.trim().is_empty() {
            continue;
        }
        let href = inline.link.as_deref()?;
        let bookmark = href.strip_prefix('#').and_then(sanitize_bookmark_id)?;
        match target.as_deref() {
            Some(existing) if existing != bookmark.as_str() => return None,
            Some(_) => {}
            None => target = Some(bookmark),
        }
    }
    let target_bookmark_id = target?;
    let text = inline_text(&paragraph.inlines)
        .trim()
        .chars()
        .take(512)
        .collect::<String>();
    if text.is_empty() {
        return None;
    }
    Some(TableOfContentsEntry {
        level: level.clamp(1, 3),
        text,
        target_bookmark_id,
    })
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
                    toc_role: None,
                    counts_for_cell_alignment: false,
                    paragraph_alignment: None,
                });
            }
        }
    }
    flush_paragraph_block(&mut blocks, &mut inlines, properties);
    if blocks.is_empty() {
        blocks.push(ParsedBlock {
            block: paragraph_block_from_inlines(
                Vec::new(),
                properties.heading_level,
                properties.bookmark_id.clone(),
                properties.format.clone(),
            ),
            list_marker: properties.list_marker,
            toc_role: properties.toc_role,
            counts_for_cell_alignment: true,
            paragraph_alignment: properties.format.alignment,
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
        block: paragraph_block_from_inlines(
            std::mem::take(inlines),
            properties.heading_level,
            properties.bookmark_id.clone(),
            properties.format.clone(),
        ),
        list_marker: properties.list_marker,
        toc_role: properties.toc_role,
        counts_for_cell_alignment: true,
        paragraph_alignment: properties.format.alignment,
    });
}

fn paragraph_block_from_inlines(
    inlines: Vec<Inline>,
    heading_level: Option<u8>,
    bookmark_id: Option<String>,
    format: ParagraphFormat,
) -> Block {
    if let Some(level) = heading_level {
        Block::Heading(Heading {
            bookmark_id,
            level,
            inlines,
        })
    } else {
        Block::Paragraph(Paragraph {
            bookmark_id,
            style: StyleId::from("body"),
            format,
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
            Event::GeneralRef(reference) => match reference.as_ref() {
                b"lt" => value.push('<'),
                b"gt" => value.push('>'),
                b"amp" => value.push('&'),
                b"quot" => value.push('"'),
                b"apos" => value.push('\''),
                _ => {}
            },
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

fn read_visible_text_fallback(
    reader: &mut Reader<&[u8]>,
    _end_name: &[u8],
    name: &str,
) -> Result<String, DocxError> {
    let mut value = String::new();
    let mut depth = 1_usize;
    loop {
        match reader.read_event().map_err(|err| xml_error(name, err))? {
            Event::Start(start)
                if matches!(local_name(start.name().as_ref()), b"t" | b"delText") =>
            {
                let end = local_name(start.name().as_ref()).to_vec();
                value.push_str(&read_text_element(reader, &end, name)?);
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"tab" => {
                value.push('\t');
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"br" => {
                value.push('\n');
            }
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    break;
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(value)
}

fn push_plain_visible_text(content: &mut Vec<ParagraphContent>, text: String) {
    if text.is_empty() {
        return;
    }
    content.push(ParagraphContent::Inline(Box::new(Inline::text(text))));
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

fn skip_table_row_properties(
    reader: &mut Reader<&[u8]>,
    warnings: &mut WarningSink,
) -> Result<bool, DocxError> {
    scan_skipped_table_properties(
        reader,
        warnings,
        is_unsupported_docx_table_row_shape_property,
    )
}

#[derive(Debug, Clone, Default)]
struct ParsedTableCellProperties {
    unsupported_width_shape: bool,
    presentation: TableCellPresentation,
}

fn parse_table_cell_properties(
    reader: &mut Reader<&[u8]>,
    warnings: &mut WarningSink,
) -> Result<ParsedTableCellProperties, DocxError> {
    let mut parsed = ParsedTableCellProperties::default();
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Empty(start) if local_name(start.name().as_ref()) == b"shd" => {
                apply_docx_table_cell_shading(&start, &mut parsed.presentation)?;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"shd" => {
                apply_docx_table_cell_shading(&start, &mut parsed.presentation)?;
                skip_element(reader, b"shd", DOCUMENT_XML)?;
            }
            Event::Start(start) if local_name(start.name().as_ref()) == b"tcBorders" => {
                parsed.presentation.border = parse_docx_table_cell_borders(reader)?;
            }
            Event::Empty(start) if local_name(start.name().as_ref()) == b"tcBorders" => {}
            Event::End(end) if local_name(end.name().as_ref()) == b"tcPr" => break,
            Event::Start(start) => {
                let name = start.name();
                let local = local_name(name.as_ref());
                if is_any_docx_revision_markup(local) {
                    warnings.warn(
                        "docx_revision_markup_degraded",
                        "Unsupported DOCX tracked-change markup was imported as visible text when available",
                    );
                }
                if is_unsupported_docx_table_cell_shape_property(local) {
                    parsed.unsupported_width_shape = true;
                }
                let end = local.to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Empty(start) => {
                let name = start.name();
                let local = local_name(name.as_ref());
                if is_any_docx_revision_markup(local) {
                    warnings.warn(
                        "docx_revision_markup_degraded",
                        "Unsupported DOCX tracked-change markup was imported as visible text when available",
                    );
                }
                if is_unsupported_docx_table_cell_shape_property(local) {
                    parsed.unsupported_width_shape = true;
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(parsed)
}

fn apply_docx_table_cell_shading(
    start: &BytesStart<'_>,
    presentation: &mut TableCellPresentation,
) -> Result<(), DocxError> {
    if let Some(value) = attr_value(start, b"val", DOCUMENT_XML)? {
        match value.as_str() {
            "nil" | "none" => return Ok(()),
            "clear" | "solid" => {}
            _ => return Ok(()),
        }
    }
    let Some(fill) = attr_value(start, b"fill", DOCUMENT_XML)? else {
        return Ok(());
    };
    let normalized = format!("#{}", fill.trim().to_ascii_lowercase());
    if let Some(color) = sanitize_table_cell_background_color(&normalized) {
        presentation.background_color = Some(color);
    }
    Ok(())
}

fn parse_docx_table_cell_borders(reader: &mut Reader<&[u8]>) -> Result<TableCellBorder, DocxError> {
    let mut hidden_sides = BTreeSet::new();
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Empty(start)
                if is_docx_table_cell_border_side(local_name(start.name().as_ref())) =>
            {
                collect_hidden_docx_border_side(&start, &mut hidden_sides)?;
            }
            Event::Start(start)
                if is_docx_table_cell_border_side(local_name(start.name().as_ref())) =>
            {
                collect_hidden_docx_border_side(&start, &mut hidden_sides)?;
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::End(end) if local_name(end.name().as_ref()) == b"tcBorders" => break,
            Event::Start(start) => {
                let end = local_name(start.name().as_ref()).to_vec();
                skip_element(reader, &end, DOCUMENT_XML)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if ["top", "left", "bottom", "right"]
        .iter()
        .all(|side| hidden_sides.contains(*side))
    {
        Ok(TableCellBorder::Hidden)
    } else {
        Ok(TableCellBorder::Visible)
    }
}

fn is_docx_table_cell_border_side(name: &[u8]) -> bool {
    matches!(name, b"top" | b"left" | b"bottom" | b"right")
}

fn collect_hidden_docx_border_side(
    start: &BytesStart<'_>,
    hidden_sides: &mut BTreeSet<&'static str>,
) -> Result<(), DocxError> {
    let Some(value) = attr_value(start, b"val", DOCUMENT_XML)? else {
        return Ok(());
    };
    if !matches!(value.as_str(), "nil" | "none") {
        return Ok(());
    }
    match local_name(start.name().as_ref()) {
        b"top" => {
            hidden_sides.insert("top");
        }
        b"left" => {
            hidden_sides.insert("left");
        }
        b"bottom" => {
            hidden_sides.insert("bottom");
        }
        b"right" => {
            hidden_sides.insert("right");
        }
        _ => {}
    }
    Ok(())
}

fn scan_skipped_table_properties(
    reader: &mut Reader<&[u8]>,
    warnings: &mut WarningSink,
    is_unsupported_shape_property: fn(&[u8]) -> bool,
) -> Result<bool, DocxError> {
    let mut depth = 1_usize;
    let mut unsupported_shape = false;
    loop {
        match reader
            .read_event()
            .map_err(|err| xml_error(DOCUMENT_XML, err))?
        {
            Event::Start(start) => {
                let name = start.name();
                let local = local_name(name.as_ref());
                if is_any_docx_revision_markup(local) {
                    warnings.warn(
                        "docx_revision_markup_degraded",
                        "Unsupported DOCX tracked-change markup was imported as visible text when available",
                    );
                }
                if is_unsupported_shape_property(local) {
                    unsupported_shape = true;
                }
                depth += 1;
            }
            Event::Empty(start) => {
                let name = start.name();
                let local = local_name(name.as_ref());
                if is_any_docx_revision_markup(local) {
                    warnings.warn(
                        "docx_revision_markup_degraded",
                        "Unsupported DOCX tracked-change markup was imported as visible text when available",
                    );
                }
                if is_unsupported_shape_property(local) {
                    unsupported_shape = true;
                }
            }
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
    Ok(unsupported_shape)
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

fn toc_role_from_style(style: &str) -> Option<TocParagraphRole> {
    if style == DOCX_TOC_TITLE_STYLE_ID {
        return Some(TocParagraphRole::Title);
    }
    let suffix = style.strip_prefix(DOCX_TOC_ENTRY_STYLE_PREFIX)?;
    let level = suffix.parse::<u8>().ok()?.clamp(1, 3);
    Some(TocParagraphRole::Entry(level))
}

fn truthy_word_bool(start: &BytesStart<'_>, name: &str) -> Result<bool, DocxError> {
    Ok(!matches!(
        attr_value(start, b"val", name)?.as_deref(),
        Some("0" | "false" | "off")
    ))
}

fn docx_tracked_change_kind(name: &[u8]) -> Option<TrackedChangeKind> {
    match name {
        b"ins" => Some(TrackedChangeKind::Insertion),
        b"del" => Some(TrackedChangeKind::Deletion),
        _ => None,
    }
}

fn docx_note_reference_kind(name: &[u8]) -> Option<NoteKind> {
    match name {
        b"footnoteReference" => Some(NoteKind::Footnote),
        b"endnoteReference" => Some(NoteKind::Endnote),
        _ => None,
    }
}

fn docx_note_element_name(kind: NoteKind) -> &'static [u8] {
    match kind {
        NoteKind::Footnote => b"footnote",
        NoteKind::Endnote => b"endnote",
    }
}

fn is_docx_note_separator(start: &BytesStart<'_>) -> Result<bool, DocxError> {
    Ok(matches!(
        attr_value(start, b"type", DOCX_NOTES_XML)?.as_deref(),
        Some("separator" | "continuationSeparator" | "continuationNotice")
    ))
}

fn docx_note_key(kind: NoteKind, raw_id: &str) -> String {
    let prefix = match kind {
        NoteKind::Footnote => "footnote",
        NoteKind::Endnote => "endnote",
    };
    format!("{prefix}:{raw_id}")
}

fn next_imported_docx_note_id(kind: NoteKind, index: usize) -> String {
    let kind = match kind {
        NoteKind::Footnote => "footnote",
        NoteKind::Endnote => "endnote",
    };
    let candidate = format!("note-docx-{kind}-{index}");
    validate_note_id(&candidate).unwrap_or(candidate)
}

fn docx_note_reference_fallback_text(kind: NoteKind) -> &'static str {
    match kind {
        NoteKind::Footnote => "[footnote]",
        NoteKind::Endnote => "[endnote]",
    }
}

fn is_any_docx_revision_markup(name: &[u8]) -> bool {
    docx_tracked_change_kind(name).is_some() || is_unsupported_docx_revision_markup(name)
}

fn is_unsupported_docx_table_row_shape_property(name: &[u8]) -> bool {
    matches!(name, b"gridBefore" | b"gridAfter")
}

fn is_unsupported_docx_table_cell_shape_property(name: &[u8]) -> bool {
    matches!(name, b"gridSpan" | b"hMerge" | b"vMerge")
}

fn is_unsupported_docx_revision_markup(name: &[u8]) -> bool {
    matches!(
        name,
        b"moveFrom"
            | b"moveTo"
            | b"moveFromRangeStart"
            | b"moveFromRangeEnd"
            | b"moveToRangeStart"
            | b"moveToRangeEnd"
            | b"moveFromRun"
            | b"moveToRun"
            | b"rPrChange"
            | b"pPrChange"
            | b"tblPrChange"
            | b"trPrChange"
            | b"tcPrChange"
            | b"sectPrChange"
            | b"numberingChange"
            | b"customXmlInsRangeStart"
            | b"customXmlInsRangeEnd"
            | b"customXmlDelRangeStart"
            | b"customXmlDelRangeEnd"
            | b"customXmlMoveFromRangeStart"
            | b"customXmlMoveFromRangeEnd"
            | b"customXmlMoveToRangeStart"
            | b"customXmlMoveToRangeEnd"
    )
}

fn next_imported_docx_tracked_change_id(index: usize) -> String {
    let candidate = format!("chg-docx-change-{index}");
    validate_tracked_change_id(&candidate).unwrap_or(candidate)
}

fn safe_imported_docx_revision_author(
    raw_author: Option<&str>,
    warnings: &mut WarningSink,
) -> String {
    let Some(raw_author) = raw_author.map(str::trim).filter(|value| !value.is_empty()) else {
        return IMPORTED_DOCX_REVISION_AUTHOR.to_string();
    };
    let Ok(author) = normalize_comment_author(Some(raw_author)) else {
        warnings.warn(
            "docx_revision_metadata_degraded",
            "Unsupported DOCX tracked-change metadata was replaced during import",
        );
        return IMPORTED_DOCX_REVISION_AUTHOR.to_string();
    };
    if author_looks_private(&author) {
        warnings.warn(
            "docx_revision_metadata_degraded",
            "Unsupported DOCX tracked-change metadata was replaced during import",
        );
        IMPORTED_DOCX_REVISION_AUTHOR.to_string()
    } else {
        author
    }
}

fn safe_docx_revision_timestamp(raw_date: Option<String>) -> DateTime<Utc> {
    raw_date
        .as_deref()
        .map(str::trim)
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(epoch_utc)
}

fn epoch_utc() -> DateTime<Utc> {
    DateTime::<Utc>::from(std::time::UNIX_EPOCH)
}

fn safe_exported_revision_author(author: &str) -> String {
    let Ok(author) = normalize_comment_author(Some(author)) else {
        return DEFAULT_TRACKED_CHANGE_AUTHOR.to_string();
    };
    if author_looks_private(&author) {
        DEFAULT_TRACKED_CHANGE_AUTHOR.to_string()
    } else {
        author
    }
}

fn author_looks_private(author: &str) -> bool {
    let trimmed = author.trim();
    let lower = trimmed.to_ascii_lowercase();
    trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains(':')
        || trimmed.contains('@')
        || lower.contains("://")
        || lower.ends_with(".local")
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
    comments: &DocxCommentExports,
    notes: &DocxNoteExports,
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
    if comments.has_comments() {
        output.push_str(
            r#"
  <Override PartName="/word/comments.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml"/>"#,
        );
    }
    if notes.has_footnotes() {
        output.push_str(
            r#"
  <Override PartName="/word/footnotes.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml"/>"#,
        );
    }
    if notes.has_endnotes() {
        output.push_str(
            r#"
  <Override PartName="/word/endnotes.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.endnotes+xml"/>"#,
        );
    }
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
    comments: &DocxCommentExports,
    notes: &DocxNoteExports,
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
    if let Some(rel_id) = comments.rel_id.as_deref() {
        output.push_str("\n  <Relationship Id=\"");
        output.push_str(&escape_xml(rel_id));
        output.push_str("\" Type=\"");
        output.push_str(REL_TYPE_COMMENTS);
        output.push_str("\" Target=\"comments.xml\"/>");
    }
    if let Some(rel_id) = notes.rel_id_footnotes.as_deref() {
        output.push_str("\n  <Relationship Id=\"");
        output.push_str(&escape_xml(rel_id));
        output.push_str("\" Type=\"");
        output.push_str(REL_TYPE_FOOTNOTES);
        output.push_str("\" Target=\"footnotes.xml\"/>");
    }
    if let Some(rel_id) = notes.rel_id_endnotes.as_deref() {
        output.push_str("\n  <Relationship Id=\"");
        output.push_str(&escape_xml(rel_id));
        output.push_str("\" Type=\"");
        output.push_str(REL_TYPE_ENDNOTES);
        output.push_str("\" Target=\"endnotes.xml\"/>");
    }
    output.push_str("\n</Relationships>");
    output
}

fn render_document_xml(
    document: &Document,
    context: &DocxRenderContext<'_>,
    page_regions: &DocxPageRegionExports,
) -> String {
    let mut output = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
<w:body>"#,
    );

    for section in &document.sections {
        for block in &section.blocks {
            render_block_xml(block, context, &mut output);
        }
    }

    output.push_str(r#"<w:sectPr>"#);
    render_section_page_region_refs(page_regions, &mut output);
    render_section_page_setup_xml(
        document.sections.first().map(|section| &section.page),
        &mut output,
    );
    output.push_str("</w:sectPr>");
    output.push_str("</w:body></w:document>");
    output
}

fn render_section_page_setup_xml(page_setup: Option<&PageSetup>, output: &mut String) {
    let default_page_setup = PageSetup::default();
    let page_setup = match page_setup {
        Some(page_setup) if page_setup.validate().is_ok() => page_setup,
        _ => &default_page_setup,
    };
    output.push_str("<w:pgSz w:w=\"");
    output.push_str(&mm_to_docx_twips(page_setup.width_mm).to_string());
    output.push_str("\" w:h=\"");
    output.push_str(&mm_to_docx_twips(page_setup.height_mm).to_string());
    output.push_str("\"/><w:pgMar w:top=\"");
    output.push_str(&mm_to_docx_twips(page_setup.margin_top_mm).to_string());
    output.push_str("\" w:right=\"");
    output.push_str(&mm_to_docx_twips(page_setup.margin_right_mm).to_string());
    output.push_str("\" w:bottom=\"");
    output.push_str(&mm_to_docx_twips(page_setup.margin_bottom_mm).to_string());
    output.push_str("\" w:left=\"");
    output.push_str(&mm_to_docx_twips(page_setup.margin_left_mm).to_string());
    output.push_str(r#"" w:header="720" w:footer="720" w:gutter="0"/>"#);
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
            render_inlines_xml(
                &paragraph.inlines,
                &HyperlinkIds::default(),
                &DocxCommentExports::default(),
                &DocxNoteExports::default(),
                &DocxRevisionExports::default(),
                output,
            );
            output.push_str("</w:p>");
        }
    }
}

fn render_block_xml(block: &Block, context: &DocxRenderContext<'_>, output: &mut String) {
    match block {
        Block::Paragraph(paragraph) => render_paragraph_xml(paragraph, None, context, output),
        Block::Heading(heading) => render_heading_xml(heading, None, context, output),
        Block::List(list) => render_list_xml(list, None, context, output),
        Block::Table(table) => render_table_xml(table, context, output),
        Block::TableOfContents(table_of_contents) => {
            render_table_of_contents_xml(table_of_contents, context, output)
        }
        Block::Image(image) => {
            if let Some(export) = docx_image_export_for(context.images, &image.asset_id) {
                render_image_xml(image, export, output);
                if let Some(caption) = image
                    .presentation
                    .caption
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    render_fallback_paragraph(caption.trim(), context, output);
                }
            } else {
                render_fallback_paragraph(&image_fallback_text(image), context, output);
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

fn render_table_of_contents_xml(
    table_of_contents: &TableOfContents,
    context: &DocxRenderContext<'_>,
    output: &mut String,
) {
    if !table_of_contents.title.trim().is_empty() {
        render_styled_paragraph_xml(
            DOCX_TOC_TITLE_STYLE_ID,
            vec![Inline::text(table_of_contents.title.trim().to_string())],
            context,
            output,
        );
    }

    for entry in &table_of_contents.entries {
        let level = entry.level.clamp(1, 3);
        let style_id = format!("{DOCX_TOC_ENTRY_STYLE_PREFIX}{level}");
        let mut inline = Inline::text(entry.text.clone());
        if let Some(target) = sanitize_bookmark_id(&entry.target_bookmark_id) {
            if context.bookmarks.numeric_id(Some(&target)).is_some() {
                inline.link = Some(format!("#{target}"));
            }
        }
        render_styled_paragraph_xml(&style_id, vec![inline], context, output);
    }
}

fn render_styled_paragraph_xml(
    style_id: &str,
    inlines: Vec<Inline>,
    context: &DocxRenderContext<'_>,
    output: &mut String,
) {
    output.push_str("<w:p><w:pPr><w:pStyle w:val=\"");
    output.push_str(&escape_xml(style_id));
    output.push_str("\"/></w:pPr>");
    render_inlines_xml(
        &inlines,
        context.hyperlinks,
        context.comments,
        context.notes,
        context.revisions,
        output,
    );
    output.push_str("</w:p>");
}

fn render_heading_xml(
    heading: &Heading,
    alignment_override: Option<ParagraphAlignment>,
    context: &DocxRenderContext<'_>,
    output: &mut String,
) {
    output.push_str("<w:p><w:pPr><w:pStyle w:val=\"Heading");
    output.push_str(&heading.level.clamp(1, 3).to_string());
    output.push_str("\"/>");
    if let Some(alignment) = alignment_override {
        output.push_str("<w:jc w:val=\"");
        output.push_str(docx_alignment_value(alignment));
        output.push_str("\"/>");
    }
    output.push_str("</w:pPr>");
    render_bookmark_start_xml(context.bookmarks, heading.bookmark_id.as_deref(), output);
    render_inlines_xml(
        &heading.inlines,
        context.hyperlinks,
        context.comments,
        context.notes,
        context.revisions,
        output,
    );
    render_bookmark_end_xml(context.bookmarks, heading.bookmark_id.as_deref(), output);
    output.push_str("</w:p>");
}

fn render_paragraph_xml(
    paragraph: &Paragraph,
    list_marker: Option<ListMarker>,
    context: &DocxRenderContext<'_>,
    output: &mut String,
) {
    render_paragraph_xml_with_alignment(paragraph, list_marker, None, context, output);
}

fn render_paragraph_xml_with_alignment(
    paragraph: &Paragraph,
    list_marker: Option<ListMarker>,
    alignment_override: Option<ParagraphAlignment>,
    context: &DocxRenderContext<'_>,
    output: &mut String,
) {
    output.push_str("<w:p>");
    let alignment = alignment_override.or(paragraph.format.alignment);
    if list_marker.is_some()
        || alignment.is_some()
        || has_docx_paragraph_spacing(&paragraph.format)
        || has_docx_paragraph_indent(&paragraph.format)
    {
        output.push_str("<w:pPr>");
        if let Some(marker) = list_marker {
            output.push_str("<w:numPr><w:ilvl w:val=\"");
            output.push_str(&marker.level.saturating_sub(1).to_string());
            output.push_str("\"/><w:numId w:val=\"");
            output.push_str(if marker.ordered { "2" } else { "1" });
            output.push_str("\"/></w:numPr>");
        }
        render_docx_paragraph_spacing(&paragraph.format, output);
        render_docx_paragraph_indent(&paragraph.format, output);
        if let Some(alignment) = alignment {
            output.push_str("<w:jc w:val=\"");
            output.push_str(docx_alignment_value(alignment));
            output.push_str("\"/>");
        }
        output.push_str("</w:pPr>");
    }
    render_bookmark_start_xml(context.bookmarks, paragraph.bookmark_id.as_deref(), output);
    render_inlines_xml(
        &paragraph.inlines,
        context.hyperlinks,
        context.comments,
        context.notes,
        context.revisions,
        output,
    );
    render_bookmark_end_xml(context.bookmarks, paragraph.bookmark_id.as_deref(), output);
    output.push_str("</w:p>");
}

fn has_docx_paragraph_spacing(format: &ParagraphFormat) -> bool {
    format.line_spacing_per_mille.is_some_and(|value| {
        (MIN_DOCX_LINE_SPACING_PER_MILLE..=MAX_DOCX_LINE_SPACING_PER_MILLE).contains(&value)
    }) || format
        .spacing_before_mm
        .is_some_and(|value| value <= MAX_DOCX_PARAGRAPH_SPACING_MM)
        || format
            .spacing_after_mm
            .is_some_and(|value| value <= MAX_DOCX_PARAGRAPH_SPACING_MM)
}

fn render_docx_paragraph_spacing(format: &ParagraphFormat, output: &mut String) {
    if !has_docx_paragraph_spacing(format) {
        return;
    }
    output.push_str("<w:spacing");
    if let Some(line_spacing) = format.line_spacing_per_mille.filter(|value| {
        (MIN_DOCX_LINE_SPACING_PER_MILLE..=MAX_DOCX_LINE_SPACING_PER_MILLE).contains(value)
    }) {
        output.push_str(" w:line=\"");
        output.push_str(&per_mille_to_docx_line_spacing(line_spacing).to_string());
        output.push_str("\" w:lineRule=\"auto\"");
    }
    if let Some(spacing_before) = format
        .spacing_before_mm
        .filter(|value| *value <= MAX_DOCX_PARAGRAPH_SPACING_MM)
    {
        output.push_str(" w:before=\"");
        output.push_str(&mm_to_docx_twips(spacing_before).to_string());
        output.push('"');
    }
    if let Some(spacing_after) = format
        .spacing_after_mm
        .filter(|value| *value <= MAX_DOCX_PARAGRAPH_SPACING_MM)
    {
        output.push_str(" w:after=\"");
        output.push_str(&mm_to_docx_twips(spacing_after).to_string());
        output.push('"');
    }
    output.push_str("/>");
}

fn has_docx_paragraph_indent(format: &ParagraphFormat) -> bool {
    format
        .indent_start_mm
        .is_some_and(|value| value <= MAX_DOCX_PARAGRAPH_INDENT_MM)
        || format
            .indent_end_mm
            .is_some_and(|value| value <= MAX_DOCX_PARAGRAPH_INDENT_MM)
        || format
            .first_line_indent_mm
            .is_some_and(docx_first_line_indent_in_bounds)
}

fn render_docx_paragraph_indent(format: &ParagraphFormat, output: &mut String) {
    if !has_docx_paragraph_indent(format) {
        return;
    }
    output.push_str("<w:ind");
    if let Some(indent_start) = format
        .indent_start_mm
        .filter(|value| *value <= MAX_DOCX_PARAGRAPH_INDENT_MM)
    {
        output.push_str(" w:left=\"");
        output.push_str(&mm_to_docx_twips(indent_start).to_string());
        output.push('"');
    }
    if let Some(indent_end) = format
        .indent_end_mm
        .filter(|value| *value <= MAX_DOCX_PARAGRAPH_INDENT_MM)
    {
        output.push_str(" w:right=\"");
        output.push_str(&mm_to_docx_twips(indent_end).to_string());
        output.push('"');
    }
    if let Some(first_line_indent) = format
        .first_line_indent_mm
        .filter(|value| docx_first_line_indent_in_bounds(*value))
    {
        if first_line_indent < 0 {
            output.push_str(" w:hanging=\"");
        } else {
            output.push_str(" w:firstLine=\"");
        }
        output.push_str(&signed_mm_to_docx_twips(first_line_indent).to_string());
        output.push('"');
    }
    output.push_str("/>");
}

fn docx_first_line_indent_in_bounds(value: i16) -> bool {
    i32::from(value).abs() <= i32::from(MAX_DOCX_FIRST_LINE_INDENT_MM)
}

fn docx_alignment_value(alignment: ParagraphAlignment) -> &'static str {
    match alignment {
        ParagraphAlignment::Left => "left",
        ParagraphAlignment::Center => "center",
        ParagraphAlignment::Right => "right",
        ParagraphAlignment::Justify => "both",
    }
}

fn render_list_xml(
    list: &ListBlock,
    alignment_override: Option<ParagraphAlignment>,
    context: &DocxRenderContext<'_>,
    output: &mut String,
) {
    let ordered = list.definition_id == "900w-ordered";
    for item in &list.items {
        for block in &item.blocks {
            match block {
                Block::Paragraph(paragraph) => render_paragraph_xml_with_alignment(
                    paragraph,
                    Some(ListMarker {
                        ordered,
                        level: item.level.clamp(1, 9),
                    }),
                    alignment_override,
                    context,
                    output,
                ),
                Block::Heading(heading) => {
                    let paragraph = Paragraph {
                        bookmark_id: None,
                        style: StyleId::from("body"),
                        format: Default::default(),
                        inlines: heading.inlines.clone(),
                    };
                    render_paragraph_xml_with_alignment(
                        &paragraph,
                        Some(ListMarker {
                            ordered,
                            level: item.level.clamp(1, 9),
                        }),
                        alignment_override,
                        context,
                        output,
                    );
                }
                Block::Image(image) => {
                    render_block_xml(&Block::Image(image.clone()), context, output)
                }
                _ => render_fallback_paragraph(&block_text(block), context, output),
            }
        }
    }
}

fn render_table_xml(table: &Table, context: &DocxRenderContext<'_>, output: &mut String) {
    output.push_str("<w:tbl><w:tblPr><w:tblW w:w=\"0\" w:type=\"auto\"/></w:tblPr>");
    render_table_grid_xml(table, output);
    for row in &table.rows {
        output.push_str("<w:tr>");
        for cell in &row.cells {
            output.push_str("<w:tc><w:tcPr><w:tcW w:w=\"0\" w:type=\"auto\"/>");
            render_table_cell_presentation_xml(&cell.presentation, output);
            output.push_str("</w:tcPr>");
            if cell.blocks.is_empty() {
                output.push_str("<w:p/>");
            } else {
                for block in &cell.blocks {
                    match block {
                        Block::Paragraph(paragraph) => render_paragraph_xml_with_alignment(
                            paragraph,
                            None,
                            cell.presentation.text_alignment,
                            context,
                            output,
                        ),
                        Block::Heading(heading) => render_heading_xml(
                            heading,
                            cell.presentation.text_alignment,
                            context,
                            output,
                        ),
                        Block::List(list) => {
                            render_list_xml(list, cell.presentation.text_alignment, context, output)
                        }
                        Block::Image(image) => {
                            render_block_xml(&Block::Image(image.clone()), context, output)
                        }
                        _ => render_fallback_paragraph(&block_text(block), context, output),
                    }
                }
            }
            output.push_str("</w:tc>");
        }
        output.push_str("</w:tr>");
    }
    output.push_str("</w:tbl>");
}

fn render_table_grid_xml(table: &Table, output: &mut String) {
    let Some(widths) = table.sanitized_column_widths() else {
        return;
    };
    output.push_str("<w:tblGrid>");
    for width in widths {
        output.push_str("<w:gridCol w:w=\"");
        output.push_str(&((u32::from(width) * DOCX_TABLE_GRID_TOTAL_DXA) / 1000).to_string());
        output.push_str("\"/>");
    }
    output.push_str("</w:tblGrid>");
}

fn render_table_cell_presentation_xml(presentation: &TableCellPresentation, output: &mut String) {
    if let Some(fill) = presentation
        .background_color
        .as_deref()
        .and_then(docx_table_cell_fill)
    {
        output.push_str("<w:shd w:val=\"clear\" w:fill=\"");
        output.push_str(fill);
        output.push_str("\"/>");
    }
    if presentation.border == TableCellBorder::Hidden {
        output.push_str("<w:tcBorders>");
        for side in ["top", "left", "bottom", "right"] {
            output.push_str("<w:");
            output.push_str(side);
            output.push_str(" w:val=\"nil\"/>");
        }
        output.push_str("</w:tcBorders>");
    }
}

fn docx_table_cell_fill(color: &str) -> Option<&'static str> {
    match sanitize_table_cell_background_color(color)?.as_str() {
        "#f1f5f9" => Some("F1F5F9"),
        "#fff3bf" => Some("FFF3BF"),
        "#dbeafe" => Some("DBEAFE"),
        "#dcfce7" => Some("DCFCE7"),
        _ => None,
    }
}

fn render_fallback_paragraph(text: &str, context: &DocxRenderContext<'_>, output: &mut String) {
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
        context,
        output,
    );
}

fn render_bookmark_start_xml(
    bookmarks: &DocxBookmarkExports,
    bookmark_id: Option<&str>,
    output: &mut String,
) {
    let Some(name) = bookmark_id.and_then(sanitize_bookmark_id) else {
        return;
    };
    let Some(numeric_id) = bookmarks.numeric_id(Some(&name)) else {
        return;
    };
    output.push_str("<w:bookmarkStart w:id=\"");
    output.push_str(&numeric_id.to_string());
    output.push_str("\" w:name=\"");
    output.push_str(&escape_xml(&name));
    output.push_str("\"/>");
}

fn render_bookmark_end_xml(
    bookmarks: &DocxBookmarkExports,
    bookmark_id: Option<&str>,
    output: &mut String,
) {
    let Some(numeric_id) = bookmarks.numeric_id(bookmark_id) else {
        return;
    };
    output.push_str("<w:bookmarkEnd w:id=\"");
    output.push_str(&numeric_id.to_string());
    output.push_str("\"/>");
}

fn render_inlines_xml(
    inlines: &[Inline],
    hyperlinks: &HyperlinkIds,
    comments: &DocxCommentExports,
    notes: &DocxNoteExports,
    revisions: &DocxRevisionExports,
    output: &mut String,
) {
    if inlines.is_empty() {
        return;
    }
    let mut previous_comment_ids = Vec::new();
    for (index, inline) in inlines.iter().enumerate() {
        let current_comment_ids = comments.ids_for_inline(inline);
        let next_comment_ids = inlines
            .get(index + 1)
            .map(|next| comments.ids_for_inline(next))
            .unwrap_or_default();
        render_comment_range_starts_xml(
            &comment_ids_entering(&current_comment_ids, &previous_comment_ids),
            comments,
            output,
        );
        if let Some(href) = inline.link.as_deref().and_then(sanitize_text_href) {
            if let Some(anchor) = href.strip_prefix('#') {
                output.push_str("<w:hyperlink w:anchor=\"");
                output.push_str(&escape_xml(anchor));
                output.push_str("\">");
                render_inline_xml(inline, notes, revisions, output);
                output.push_str("</w:hyperlink>");
                render_comment_range_ends_xml(
                    &comment_ids_exiting(&current_comment_ids, &next_comment_ids),
                    comments,
                    output,
                );
                previous_comment_ids = current_comment_ids;
                continue;
            }
            if let Some(id) = hyperlinks.external.get(&href) {
                output.push_str("<w:hyperlink r:id=\"");
                output.push_str(&escape_xml(id));
                output.push_str("\" w:history=\"1\">");
                render_inline_xml(inline, notes, revisions, output);
                output.push_str("</w:hyperlink>");
                render_comment_range_ends_xml(
                    &comment_ids_exiting(&current_comment_ids, &next_comment_ids),
                    comments,
                    output,
                );
                previous_comment_ids = current_comment_ids;
                continue;
            }
        }
        render_inline_xml(inline, notes, revisions, output);
        render_comment_range_ends_xml(
            &comment_ids_exiting(&current_comment_ids, &next_comment_ids),
            comments,
            output,
        );
        previous_comment_ids = current_comment_ids;
    }
}

fn comment_ids_entering(current: &[String], previous: &[String]) -> Vec<String> {
    current
        .iter()
        .filter(|id| !previous.contains(*id))
        .cloned()
        .collect()
}

fn comment_ids_exiting(current: &[String], next: &[String]) -> Vec<String> {
    current
        .iter()
        .filter(|id| !next.contains(*id))
        .cloned()
        .collect()
}

fn render_comment_range_starts_xml(
    comment_ids: &[String],
    comments: &DocxCommentExports,
    output: &mut String,
) {
    for id in comment_ids {
        if let Some(numeric_id) = comments.numeric_id(id) {
            output.push_str("<w:commentRangeStart w:id=\"");
            output.push_str(&numeric_id.to_string());
            output.push_str("\"/>");
        }
    }
}

fn render_comment_range_ends_xml(
    comment_ids: &[String],
    comments: &DocxCommentExports,
    output: &mut String,
) {
    for id in comment_ids.iter().rev() {
        if let Some(numeric_id) = comments.numeric_id(id) {
            output.push_str("<w:commentRangeEnd w:id=\"");
            output.push_str(&numeric_id.to_string());
            output.push_str("\"/><w:r><w:commentReference w:id=\"");
            output.push_str(&numeric_id.to_string());
            output.push_str("\"/></w:r>");
        }
    }
}

fn render_inline_xml(
    inline: &Inline,
    notes: &DocxNoteExports,
    revisions: &DocxRevisionExports,
    output: &mut String,
) {
    if let Some(reference) = inline.note_reference.as_ref() {
        if let Some(numeric_id) = notes.numeric_id(reference) {
            render_note_reference_xml(inline, reference.kind, numeric_id, output);
            return;
        }
    }
    if let Some(change) = exportable_docx_revision_change(inline) {
        if let Some(numeric_id) = revisions.numeric_id(&change.id) {
            render_tracked_change_xml(inline, change, numeric_id, output);
            return;
        }
    }
    render_run_xml_with_text_element(inline, "t", output);
}

fn render_note_reference_xml(
    inline: &Inline,
    kind: NoteKind,
    numeric_id: u32,
    output: &mut String,
) {
    output.push_str("<w:r>");
    render_run_properties_xml(inline, output);
    output.push_str(match kind {
        NoteKind::Footnote => "<w:footnoteReference w:id=\"",
        NoteKind::Endnote => "<w:endnoteReference w:id=\"",
    });
    output.push_str(&numeric_id.to_string());
    output.push_str("\"/></w:r>");
}

fn render_tracked_change_xml(
    inline: &Inline,
    change: &TrackedChange,
    numeric_id: u32,
    output: &mut String,
) {
    let element = match change.kind {
        TrackedChangeKind::Insertion => "ins",
        TrackedChangeKind::Deletion => "del",
    };
    output.push_str("<w:");
    output.push_str(element);
    output.push_str(" w:id=\"");
    output.push_str(&numeric_id.to_string());
    output.push_str("\" w:author=\"");
    output.push_str(&escape_xml(&safe_exported_revision_author(&change.author)));
    output.push_str("\" w:date=\"");
    output.push_str(&escape_xml(
        &change.created_at.to_rfc3339_opts(SecondsFormat::Secs, true),
    ));
    output.push_str("\">");
    render_run_xml_with_text_element(
        inline,
        match change.kind {
            TrackedChangeKind::Insertion => "t",
            TrackedChangeKind::Deletion => "delText",
        },
        output,
    );
    output.push_str("</w:");
    output.push_str(element);
    output.push('>');
}

fn render_run_xml_with_text_element(inline: &Inline, text_element: &str, output: &mut String) {
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
        fallback.style = inline.style.clone();
        render_run_xml_with_text_element(&fallback, text_element, output);
        output.push_str("</w:fldSimple>");
        return;
    }
    if inline.text.is_empty() {
        return;
    }
    output.push_str("<w:r>");
    render_run_properties_xml(inline, output);

    let mut text_buffer = String::new();
    for ch in inline.text.chars() {
        match ch {
            '\n' => {
                flush_text_run(&mut text_buffer, text_element, output);
                output.push_str("<w:br/>");
            }
            '\t' => {
                flush_text_run(&mut text_buffer, text_element, output);
                output.push_str("<w:tab/>");
            }
            _ => text_buffer.push(ch),
        }
    }
    flush_text_run(&mut text_buffer, text_element, output);
    output.push_str("</w:r>");
}

fn render_run_properties_xml(inline: &Inline, output: &mut String) {
    let marks = &inline.marks;
    if !marks.is_empty() || has_exportable_docx_inline_style(&inline.style) {
        output.push_str("<w:rPr>");
        if marks.contains(&InlineMark::Bold) {
            output.push_str("<w:b/>");
        }
        if marks.contains(&InlineMark::Italic) {
            output.push_str("<w:i/>");
        }
        if marks.contains(&InlineMark::Underline) {
            output.push_str("<w:u w:val=\"single\"/>");
        }
        if marks.contains(&InlineMark::Strikethrough) {
            output.push_str("<w:strike/>");
        }
        if marks.contains(&InlineMark::Superscript) {
            output.push_str("<w:vertAlign w:val=\"superscript\"/>");
        } else if marks.contains(&InlineMark::Subscript) {
            output.push_str("<w:vertAlign w:val=\"subscript\"/>");
        }
        if let Some(font_size) = inline
            .style
            .font_size_pt
            .filter(|value| SUPPORTED_DOCX_INLINE_FONT_SIZES_PT.contains(value))
        {
            output.push_str("<w:sz w:val=\"");
            output.push_str(&(font_size * 2).to_string());
            output.push_str("\"/>");
        }
        if let Some(color) = inline
            .style
            .text_color
            .as_deref()
            .and_then(docx_text_color_value)
        {
            output.push_str("<w:color w:val=\"");
            output.push_str(&color);
            output.push_str("\"/>");
        }
        if let Some(highlight) = inline
            .style
            .highlight_color
            .as_deref()
            .and_then(docx_highlight_value)
        {
            output.push_str("<w:highlight w:val=\"");
            output.push_str(highlight);
            output.push_str("\"/>");
        }
        output.push_str("</w:rPr>");
    }
}

fn has_exportable_docx_inline_style(style: &InlineStyle) -> bool {
    style
        .font_size_pt
        .is_some_and(|value| SUPPORTED_DOCX_INLINE_FONT_SIZES_PT.contains(&value))
        || style
            .text_color
            .as_deref()
            .and_then(docx_text_color_value)
            .is_some()
        || style
            .highlight_color
            .as_deref()
            .and_then(docx_highlight_value)
            .is_some()
}

fn flush_text_run(text: &mut String, text_element: &str, output: &mut String) {
    if text.is_empty() {
        return;
    }
    output.push_str("<w:");
    output.push_str(text_element);
    output.push_str(" xml:space=\"preserve\">");
    output.push_str(&escape_xml(text));
    output.push_str("</w:");
    output.push_str(text_element);
    output.push('>');
    text.clear();
}

fn render_styles_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
  <w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/></w:pPr><w:rPr><w:b/><w:sz w:val="32"/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:qFormat/><w:rPr><w:b/><w:sz w:val="28"/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="Heading3"><w:name w:val="heading 3"/><w:basedOn w:val="Normal"/><w:qFormat/><w:rPr><w:b/><w:sz w:val="24"/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="Word900TocTitle"><w:name w:val="900Word TOC title"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:before="120" w:after="80"/></w:pPr><w:rPr><w:b/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="Word900TocEntry1"><w:name w:val="900Word TOC entry 1"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="40"/></w:pPr></w:style>
  <w:style w:type="paragraph" w:styleId="Word900TocEntry2"><w:name w:val="900Word TOC entry 2"/><w:basedOn w:val="Normal"/><w:pPr><w:ind w:left="360"/><w:spacing w:after="40"/></w:pPr></w:style>
  <w:style w:type="paragraph" w:styleId="Word900TocEntry3"><w:name w:val="900Word TOC entry 3"/><w:basedOn w:val="Normal"/><w:pPr><w:ind w:left="720"/><w:spacing w:after="40"/></w:pPr></w:style>
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

fn render_comments_xml(comments: &DocxCommentExports) -> String {
    let mut output = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
    );
    for comment in &comments.comments {
        output.push_str("<w:comment w:id=\"");
        output.push_str(&comment.numeric_id.to_string());
        output.push_str("\" w:author=\"");
        output.push_str(&escape_xml(&comment.author));
        output.push_str("\" w:date=\"");
        output.push_str(&escape_xml(
            &comment
                .created_at
                .to_rfc3339_opts(SecondsFormat::Secs, true),
        ));
        output.push_str("\"><w:p><w:r><w:t xml:space=\"preserve\">");
        output.push_str(&escape_xml(&comment.body));
        output.push_str("</w:t></w:r></w:p></w:comment>");
    }
    output.push_str("</w:comments>");
    output
}

fn render_notes_xml(kind: NoteKind, notes: &[DocxNoteExport]) -> String {
    let root = match kind {
        NoteKind::Footnote => "footnotes",
        NoteKind::Endnote => "endnotes",
    };
    let element = match kind {
        NoteKind::Footnote => "footnote",
        NoteKind::Endnote => "endnote",
    };
    let reference = match kind {
        NoteKind::Footnote => "footnoteRef",
        NoteKind::Endnote => "endnoteRef",
    };
    let mut output = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    output.push_str("<w:");
    output.push_str(root);
    output.push_str(r#" xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#);
    output.push_str("<w:");
    output.push_str(element);
    output.push_str(r#" w:type="separator" w:id="-1"><w:p><w:r><w:separator/></w:r></w:p></w:"#);
    output.push_str(element);
    output.push_str("><w:");
    output.push_str(element);
    output.push_str(r#" w:type="continuationSeparator" w:id="0"><w:p><w:r><w:continuationSeparator/></w:r></w:p></w:"#);
    output.push_str(element);
    output.push('>');
    for note in notes {
        output.push_str("<w:");
        output.push_str(element);
        output.push_str(" w:id=\"");
        output.push_str(&note.numeric_id.to_string());
        output.push_str("\">");
        render_note_body_xml(&note.body, reference, &mut output);
        output.push_str("</w:");
        output.push_str(element);
        output.push('>');
    }
    output.push_str("</w:");
    output.push_str(root);
    output.push('>');
    output
}

fn render_note_body_xml(body: &str, reference: &str, output: &mut String) {
    let mut first = true;
    for line in body.split('\n') {
        output.push_str("<w:p>");
        if first {
            output.push_str("<w:r><w:");
            output.push_str(reference);
            output.push_str("/></w:r>");
        }
        if !line.is_empty() {
            render_run_xml_with_text_element(&Inline::text(line), "t", output);
        }
        output.push_str("</w:p>");
        first = false;
    }
    if first {
        output.push_str("<w:p><w:r><w:");
        output.push_str(reference);
        output.push_str("/></w:r></w:p>");
    }
}

fn collect_external_hyperlinks(document: &Document) -> BTreeSet<String> {
    let mut links = BTreeSet::new();
    for section in &document.sections {
        collect_external_hyperlinks_from_blocks(&section.blocks, &mut links);
    }
    links
}

fn collect_docx_bookmark_exports(document: &Document) -> DocxBookmarkExports {
    let mut counts = BTreeMap::new();
    for section in &document.sections {
        collect_docx_bookmark_counts_from_blocks(&section.blocks, &mut counts);
    }
    let ids = counts
        .into_iter()
        .filter(|(_, count)| *count == 1)
        .enumerate()
        .map(|(index, (bookmark_id, _))| (bookmark_id, index as u32))
        .collect();
    DocxBookmarkExports { ids }
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

fn collect_docx_comment_exports(document: &Document, first_rel_id: usize) -> DocxCommentExports {
    let anchored = collect_exportable_comment_anchor_ids(document);
    let mut ids = BTreeMap::new();
    let mut comments = Vec::new();
    for local_id in anchored {
        let Some(comment) = document.comments.get(&local_id) else {
            continue;
        };
        if validate_comment_id(&comment.id).is_err()
            || comment.id != local_id
            || validate_comment_body(&comment.body).is_err()
        {
            continue;
        }
        let Ok(author) = normalize_comment_author(Some(&comment.author)) else {
            continue;
        };
        let numeric_id = comments.len() as u32;
        ids.insert(local_id.clone(), numeric_id);
        comments.push(DocxCommentExport {
            numeric_id,
            author,
            body: comment.body.clone(),
            created_at: comment.created_at,
        });
        if comments.len() >= MAX_DOCX_COMMENTS {
            break;
        }
    }
    let rel_id = if comments.is_empty() {
        None
    } else {
        Some(format!("rId{first_rel_id}"))
    };
    DocxCommentExports {
        rel_id,
        ids,
        comments,
    }
}

fn collect_docx_note_exports(document: &Document, first_rel_id: usize) -> DocxNoteExports {
    let references = collect_ordered_note_references(&document.sections);
    let mut exports = DocxNoteExports::default();
    for reference in references {
        if exports.ids.contains_key(&reference.id) || exports.ids.len() >= MAX_NOTES {
            continue;
        }
        if validate_note_reference(&reference).is_err() {
            continue;
        }
        let Some(note) = document.notes.get(&reference.id) else {
            continue;
        };
        if note.id != reference.id || note.kind != reference.kind {
            continue;
        }
        let Ok(body) = validate_note_body(&note.body) else {
            continue;
        };
        match reference.kind {
            NoteKind::Footnote => {
                let numeric_id = exports.footnotes.len() as u32 + 1;
                exports.ids.insert(
                    reference.id.clone(),
                    DocxNoteExportId {
                        kind: reference.kind,
                        numeric_id,
                    },
                );
                exports.footnotes.push(DocxNoteExport { numeric_id, body });
            }
            NoteKind::Endnote => {
                let numeric_id = exports.endnotes.len() as u32 + 1;
                exports.ids.insert(
                    reference.id.clone(),
                    DocxNoteExportId {
                        kind: reference.kind,
                        numeric_id,
                    },
                );
                exports.endnotes.push(DocxNoteExport { numeric_id, body });
            }
        }
    }

    let mut next_rel_id = first_rel_id;
    if exports.has_footnotes() {
        exports.rel_id_footnotes = Some(format!("rId{next_rel_id}"));
        next_rel_id += 1;
    }
    if exports.has_endnotes() {
        exports.rel_id_endnotes = Some(format!("rId{next_rel_id}"));
    }
    exports
}

fn collect_docx_revision_exports(document: &Document) -> DocxRevisionExports {
    let mut ids = BTreeMap::new();
    for section in &document.sections {
        collect_docx_revision_exports_from_blocks(&section.blocks, &mut ids);
    }
    DocxRevisionExports { ids }
}

fn collect_docx_revision_exports_from_blocks(blocks: &[Block], ids: &mut BTreeMap<String, u32>) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                collect_docx_revision_exports_from_inlines(&paragraph.inlines, ids)
            }
            Block::Heading(heading) => {
                collect_docx_revision_exports_from_inlines(&heading.inlines, ids)
            }
            Block::List(list) => {
                for item in &list.items {
                    collect_docx_revision_exports_from_blocks(&item.blocks, ids);
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_docx_revision_exports_from_blocks(&cell.blocks, ids);
                    }
                }
            }
            Block::TableOfContents(_) | Block::Image(_) | Block::PageBreak => {}
        }
    }
}

fn collect_docx_revision_exports_from_inlines(inlines: &[Inline], ids: &mut BTreeMap<String, u32>) {
    for inline in inlines {
        let Some(change) = exportable_docx_revision_change(inline) else {
            continue;
        };
        if !ids.contains_key(&change.id) {
            ids.insert(change.id.clone(), ids.len() as u32);
        }
    }
}

fn exportable_docx_revision_change(inline: &Inline) -> Option<&TrackedChange> {
    let change = inline.tracked_change.as_ref()?;
    if inline.text.is_empty()
        || inline.field.is_some()
        || inline.note_reference.is_some()
        || validate_tracked_change_id(&change.id).is_err()
    {
        return None;
    }
    Some(change)
}

fn collect_exportable_comment_anchor_ids(document: &Document) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut ids = Vec::new();
    for section in &document.sections {
        collect_exportable_comment_anchor_ids_from_blocks(&section.blocks, &mut seen, &mut ids);
    }
    ids
}

fn collect_exportable_comment_anchor_ids_from_blocks(
    blocks: &[Block],
    seen: &mut BTreeSet<String>,
    ids: &mut Vec<String>,
) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                collect_exportable_comment_anchor_ids_from_inlines(&paragraph.inlines, seen, ids)
            }
            Block::Heading(heading) => {
                collect_exportable_comment_anchor_ids_from_inlines(&heading.inlines, seen, ids)
            }
            Block::List(list) => {
                for item in &list.items {
                    collect_exportable_comment_anchor_ids_from_blocks(&item.blocks, seen, ids);
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_exportable_comment_anchor_ids_from_blocks(&cell.blocks, seen, ids);
                    }
                }
            }
            Block::TableOfContents(_) | Block::Image(_) | Block::PageBreak => {}
        }
    }
}

fn collect_exportable_comment_anchor_ids_from_inlines(
    inlines: &[Inline],
    seen: &mut BTreeSet<String>,
    ids: &mut Vec<String>,
) {
    for inline in inlines {
        if inline.text.is_empty()
            || inline.field.is_some()
            || inline.note_reference.is_some()
            || inline.tracked_change.is_some()
        {
            continue;
        }
        for id in &inline.comment_ids {
            if validate_comment_id(id).is_ok() && seen.insert(id.clone()) {
                ids.push(id.clone());
            }
        }
    }
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

fn collect_docx_bookmark_counts_from_blocks(
    blocks: &[Block],
    counts: &mut BTreeMap<String, usize>,
) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                if let Some(bookmark_id) = paragraph
                    .bookmark_id
                    .as_deref()
                    .and_then(sanitize_bookmark_id)
                {
                    *counts.entry(bookmark_id).or_insert(0) += 1;
                }
            }
            Block::Heading(heading) => {
                if let Some(bookmark_id) = heading
                    .bookmark_id
                    .as_deref()
                    .and_then(sanitize_bookmark_id)
                {
                    *counts.entry(bookmark_id).or_insert(0) += 1;
                }
            }
            Block::List(list) => {
                for item in &list.items {
                    collect_docx_bookmark_counts_from_blocks(&item.blocks, counts);
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_docx_bookmark_counts_from_blocks(&cell.blocks, counts);
                    }
                }
            }
            Block::TableOfContents(_) | Block::Image(_) | Block::PageBreak => {}
        }
    }
}

fn collect_external_hyperlinks_from_inlines(inlines: &[Inline], links: &mut BTreeSet<String>) {
    for inline in inlines {
        if inline.tracked_change.is_some() {
            continue;
        }
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

fn resolve_comments_target(target: &str) -> Option<String> {
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
    if file_name.contains('/') || !file_name.starts_with("comments") || !file_name.ends_with(".xml")
    {
        return None;
    }
    if validate_entry_path(&combined, PackageLimits::default()).is_ok() {
        Some(combined)
    } else {
        None
    }
}

fn resolve_note_target(target: &str, kind: NoteKind) -> Option<String> {
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
    let expected = match kind {
        NoteKind::Footnote => "footnotes.xml",
        NoteKind::Endnote => "endnotes.xml",
    };
    if file_name == expected && validate_entry_path(&combined, PackageLimits::default()).is_ok() {
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

fn next_imported_docx_comment_id(raw_id: &str, index: usize) -> String {
    let safe = raw_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .take(32)
        .collect::<String>();
    let candidate = if safe.is_empty() {
        format!("cmt-docx-comment-{index}")
    } else {
        format!("cmt-docx-comment-{index}-{safe}")
    };
    validate_comment_id(&candidate).unwrap_or_else(|_| format!("cmt-docx-comment-{index}"))
}

fn prune_comment_ids_from_blocks(blocks: &mut [Block], anchored_comment_ids: &BTreeSet<String>) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                prune_comment_ids_from_inlines(&mut paragraph.inlines, anchored_comment_ids)
            }
            Block::Heading(heading) => {
                prune_comment_ids_from_inlines(&mut heading.inlines, anchored_comment_ids)
            }
            Block::List(list) => {
                for item in &mut list.items {
                    prune_comment_ids_from_blocks(&mut item.blocks, anchored_comment_ids);
                }
            }
            Block::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        prune_comment_ids_from_blocks(&mut cell.blocks, anchored_comment_ids);
                    }
                }
            }
            Block::TableOfContents(_) | Block::Image(_) | Block::PageBreak => {}
        }
    }
}

fn prune_comment_ids_from_inlines(inlines: &mut [Inline], anchored_comment_ids: &BTreeSet<String>) {
    for inline in inlines {
        inline
            .comment_ids
            .retain(|id| anchored_comment_ids.contains(id));
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
    fn imports_docx_inline_formatting_from_safe_subset() {
        let bytes = synthetic_docx(
            r##"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:p>
    <w:r><w:rPr><w:strike/><w:vertAlign w:val="superscript"/><w:sz w:val="28"/><w:color w:val="1F2937"/><w:highlight w:val="yellow"/></w:rPr><w:t>Styled</w:t></w:r>
    <w:r><w:rPr><w:dstrike/><w:vertAlign w:val="subscript"/><w:sz w:val="18"/><w:color w:val="0066CC"/><w:highlight w:val="cyan"/></w:rPr><w:t> small</w:t></w:r>
  </w:p>
</w:body></w:document>"##,
            None,
            None,
        );

        let document = read_docx_bytes(&bytes).expect("formatted inline content should import");
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("paragraph should import");
        };

        assert_eq!(
            paragraph.inlines[0].marks,
            vec![InlineMark::Strikethrough, InlineMark::Superscript]
        );
        assert_eq!(
            paragraph.inlines[0].style,
            InlineStyle {
                font_family: None,
                font_size_pt: Some(14),
                text_color: Some("#1f2937".to_string()),
                highlight_color: Some("#fff3bf".to_string()),
            }
        );
        assert_eq!(
            paragraph.inlines[1].marks,
            vec![InlineMark::Strikethrough, InlineMark::Subscript]
        );
        assert_eq!(paragraph.inlines[1].style.font_size_pt, Some(9));
        assert_eq!(
            paragraph.inlines[1].style.text_color.as_deref(),
            Some("#0066cc")
        );
        assert_eq!(
            paragraph.inlines[1].style.highlight_color.as_deref(),
            Some("#dbeafe")
        );
    }

    #[test]
    fn ignores_unsupported_docx_inline_formatting_values() {
        let bytes = synthetic_docx(
            r##"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:p>
    <w:r><w:rPr><w:strike w:val="false"/><w:vertAlign w:val="baseline"/><w:sz w:val="27"/><w:color w:val="auto"/><w:highlight w:val="red"/></w:rPr><w:t>Unsupported</w:t></w:r>
    <w:r><w:rPr><w:color w:val="1F2937" w:themeColor="accent1"/><w:highlight w:val="darkYellow"/></w:rPr><w:t> themed</w:t></w:r>
  </w:p>
</w:body></w:document>"##,
            None,
            None,
        );

        let document = read_docx_bytes(&bytes).expect("unsupported inline content should import");
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("paragraph should import");
        };

        assert_eq!(paragraph.inlines.len(), 1);
        assert_eq!(paragraph.inlines[0].text, "Unsupported themed");
        assert!(paragraph.inlines[0].marks.is_empty());
        assert!(paragraph.inlines[0].style.is_default());
    }

    #[test]
    fn imports_docx_strike_and_double_strike_independently() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:p>
    <w:r><w:rPr><w:strike/><w:dstrike w:val="false"/></w:rPr><w:t>Strike</w:t></w:r>
    <w:r><w:rPr><w:strike w:val="false"/><w:dstrike/></w:rPr><w:t>Double</w:t></w:r>
    <w:r><w:rPr><w:strike w:val="false"/><w:dstrike w:val="false"/></w:rPr><w:t>Plain</w:t></w:r>
  </w:p>
</w:body></w:document>"#,
            None,
            None,
        );

        let document = read_docx_bytes(&bytes).expect("strike variants should import");
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("paragraph should import");
        };

        assert_eq!(paragraph.inlines.len(), 2);
        assert_eq!(paragraph.inlines[0].text, "StrikeDouble");
        assert_eq!(paragraph.inlines[0].marks, vec![InlineMark::Strikethrough]);
        assert_eq!(paragraph.inlines[1].text, "Plain");
        assert!(paragraph.inlines[1].marks.is_empty());
    }

    #[test]
    fn imports_docx_paragraph_formatting_from_safe_subset() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:p>
    <w:pPr>
      <w:spacing w:line="360" w:lineRule="auto" w:before="170" w:after="283"/>
      <w:ind w:left="454" w:right="227" w:hanging="170"/>
      <w:jc w:val="both"/>
    </w:pPr>
    <w:r><w:t>Formatted paragraph</w:t></w:r>
  </w:p>
</w:body></w:document>"#,
            None,
            None,
        );

        let document = read_docx_bytes(&bytes).expect("formatted paragraph should import");
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("paragraph should import");
        };

        assert_eq!(
            paragraph.format,
            ParagraphFormat {
                alignment: Some(ParagraphAlignment::Justify),
                line_spacing_per_mille: Some(1500),
                spacing_before_mm: Some(3),
                spacing_after_mm: Some(5),
                indent_start_mm: Some(8),
                indent_end_mm: Some(4),
                first_line_indent_mm: Some(-3),
            }
        );
    }

    #[test]
    fn ignores_unsupported_docx_paragraph_formatting_values() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:p>
    <w:pPr>
      <w:spacing w:line="480" w:lineRule="exact" w:before="999999" w:after="-1"/>
      <w:ind w:left="-1" w:right="999999" w:firstLine="999999"/>
    </w:pPr>
    <w:r><w:t>Unsupported formatting</w:t></w:r>
  </w:p>
</w:body></w:document>"#,
            None,
            None,
        );

        let document = read_docx_bytes(&bytes).expect("paragraph should import");
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("paragraph should import");
        };

        assert_eq!(paragraph.format, ParagraphFormat::default());
    }

    #[test]
    fn ignores_extreme_docx_paragraph_formatting_without_overflow() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:p>
    <w:pPr>
      <w:spacing w:line="4294967295" w:before="2147483647"/>
      <w:ind w:firstLine="-1857940166" w:hanging="-1857940166"/>
    </w:pPr>
    <w:r><w:t>Extreme formatting</w:t></w:r>
  </w:p>
</w:body></w:document>"#,
            None,
            None,
        );

        let document = read_docx_bytes(&bytes).expect("extreme formatting should not panic");
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("paragraph should import");
        };

        assert_eq!(paragraph.format, ParagraphFormat::default());
    }

    #[test]
    fn imports_docx_page_setup_from_safe_section_properties() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:p><w:r><w:t>Page setup</w:t></w:r></w:p>
  <w:sectPr>
    <w:pgSz w:w="12240" w:h="15840"/>
    <w:pgMar w:top="1134" w:right="850" w:bottom="1417" w:left="1020" w:header="720" w:footer="720" w:gutter="0"/>
  </w:sectPr>
</w:body></w:document>"#,
            None,
            None,
        );

        let document = read_docx_bytes(&bytes).expect("page setup should import");

        assert_eq!(
            document.sections[0].page,
            PageSetup {
                width_mm: 216,
                height_mm: 279,
                margin_top_mm: 20,
                margin_right_mm: 15,
                margin_bottom_mm: 25,
                margin_left_mm: 18,
            }
        );
    }

    #[test]
    fn ignores_unsupported_docx_page_setup_values() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:p><w:r><w:t>Bad page setup</w:t></w:r></w:p>
  <w:sectPr>
    <w:pgSz w:w="100" w:h="999999"/>
    <w:pgMar w:top="-1" w:right="999999" w:bottom="999999" w:left="-1"/>
  </w:sectPr>
</w:body></w:document>"#,
            None,
            None,
        );

        let document = read_docx_bytes(&bytes).expect("invalid page setup should import safely");

        assert_eq!(document.sections[0].page, PageSetup::default());
        assert!(document
            .warnings
            .iter()
            .any(|warning| warning.code == "docx_page_setup_ignored"));
    }

    #[test]
    fn ignores_partial_docx_page_setup_values() {
        for sect_pr in [
            r#"<w:sectPr><w:pgSz w:w="12240" w:h="15840"/></w:sectPr>"#,
            r#"<w:sectPr><w:pgMar w:top="1134" w:right="850" w:bottom="1417" w:left="1020"/></w:sectPr>"#,
            r#"<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1134" w:right="850" w:bottom="1417"/></w:sectPr>"#,
            r#"<w:sectPr><w:pgSz w:w="100" w:h="15840"/><w:pgMar w:top="1134" w:right="850" w:bottom="1417" w:left="1020"/></w:sectPr>"#,
        ] {
            let document_xml = format!(
                r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:p><w:r><w:t>Partial page setup</w:t></w:r></w:p>
  {sect_pr}
</w:body></w:document>"#
            );
            let bytes = synthetic_docx(&document_xml, None, None);
            let document =
                read_docx_bytes(&bytes).expect("partial page setup should import safely");

            assert_eq!(document.sections[0].page, PageSetup::default());
            assert!(document
                .warnings
                .iter()
                .any(|warning| warning.code == "docx_page_setup_ignored"));
        }
    }

    #[test]
    fn imports_body_level_docx_page_setup_as_single_section_page() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:p>
    <w:pPr>
      <w:sectPr>
        <w:pgSz w:w="11906" w:h="16838"/>
        <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/>
      </w:sectPr>
    </w:pPr>
    <w:r><w:t>Intermediate section</w:t></w:r>
  </w:p>
  <w:sectPr>
    <w:pgSz w:w="12240" w:h="15840"/>
    <w:pgMar w:top="1134" w:right="850" w:bottom="1417" w:left="1020"/>
  </w:sectPr>
</w:body></w:document>"#,
            None,
            None,
        );

        let document = read_docx_bytes(&bytes).expect("body-level page setup should import");

        assert_eq!(
            document.sections[0].page,
            PageSetup {
                width_mm: 216,
                height_mm: 279,
                margin_top_mm: 20,
                margin_right_mm: 15,
                margin_bottom_mm: 25,
                margin_left_mm: 18,
            }
        );
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
    fn imports_synthetic_docx_simple_comment_and_anchors_inline_text() {
        let bytes = synthetic_docx_with_parts(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<w:body><w:p><w:r><w:t>Before </w:t></w:r><w:commentRangeStart w:id="7"/><w:r><w:t>commented</w:t></w:r><w:commentRangeEnd w:id="7"/><w:r><w:commentReference w:id="7"/></w:r><w:r><w:t> after</w:t></w:r></w:p></w:body></w:document>"#,
            Some(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rCmt1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="comments.xml"/>
</Relationships>"#,
            ),
            None,
            &[(
                "word/comments.xml",
                r#"<w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:comment w:id="7" w:author="Reviewer" w:date="2026-06-25T10:00:00Z"><w:p><w:r><w:t>Review note</w:t></w:r></w:p></w:comment></w:comments>"#,
            )],
        );

        let document = read_docx_bytes(&bytes).expect("commented docx should import");

        assert_eq!(document.comments.len(), 1);
        let (comment_id, comment) = document
            .comments
            .iter()
            .next()
            .expect("comment should exist");
        assert_eq!(comment_id, "cmt-docx-comment-1-7");
        assert_eq!(comment.author, "Reviewer");
        assert_eq!(comment.body, "Review note");
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("paragraph expected");
        };
        assert_eq!(paragraph.inlines[1].text, "commented");
        assert_eq!(paragraph.inlines[1].comment_ids, vec![comment_id.clone()]);
        assert!(paragraph.inlines[0].comment_ids.is_empty());
        assert!(paragraph.inlines[2].comment_ids.is_empty());
        assert!(!document
            .warnings
            .iter()
            .any(|warning| warning.code == "docx_inline_metadata_ignored"));
    }

    #[test]
    fn imports_docx_comments_with_distinct_generated_ids_after_sanitizing() {
        let bytes = synthetic_docx_with_parts(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<w:body><w:p><w:commentRangeStart w:id="a!"/><w:r><w:t>One</w:t></w:r><w:commentRangeEnd w:id="a!"/><w:r><w:t> </w:t></w:r><w:commentRangeStart w:id="a@"/><w:r><w:t>Two</w:t></w:r><w:commentRangeEnd w:id="a@"/></w:p></w:body></w:document>"#,
            Some(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rCmt1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="comments.xml"/>
</Relationships>"#,
            ),
            None,
            &[(
                "word/comments.xml",
                r#"<w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:comment w:id="a!" w:author="Reviewer"><w:p><w:r><w:t>First note</w:t></w:r></w:p></w:comment><w:comment w:id="a@" w:author="Reviewer"><w:p><w:r><w:t>Second note</w:t></w:r></w:p></w:comment></w:comments>"#,
            )],
        );

        let document = read_docx_bytes(&bytes).expect("comments should import");

        assert!(document.comments.contains_key("cmt-docx-comment-1-a"));
        assert!(document.comments.contains_key("cmt-docx-comment-2-a"));
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("paragraph expected");
        };
        assert_eq!(
            paragraph.inlines[0].comment_ids,
            vec!["cmt-docx-comment-1-a"]
        );
        assert_eq!(
            paragraph.inlines[2].comment_ids,
            vec!["cmt-docx-comment-2-a"]
        );
    }

    #[test]
    fn ignores_unsafe_missing_and_remote_docx_comments_targets_with_generic_warnings() {
        for (target, target_mode, extra_parts, expected_code) in [
            (
                "../private/comments.xml",
                "",
                Vec::new(),
                "docx_comments_relationship_ignored",
            ),
            (
                "/absolute/comments.xml",
                "",
                Vec::new(),
                "docx_comments_relationship_ignored",
            ),
            (
                "C:/placeholder/comments.xml",
                "",
                Vec::new(),
                "docx_comments_relationship_ignored",
            ),
            (
                "review\\comments.xml",
                "",
                Vec::new(),
                "docx_comments_relationship_ignored",
            ),
            (
                "notes/comments.xml",
                "",
                Vec::new(),
                "docx_comments_relationship_ignored",
            ),
            (
                "comments.xml",
                r#" TargetMode="External""#,
                Vec::new(),
                "docx_comments_relationship_ignored",
            ),
            ("comments.xml", "", Vec::new(), "docx_comments_part_missing"),
        ] {
            let rels_xml = format!(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rCmt1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="{target}"{target_mode}/>
</Relationships>"#
            );
            let bytes = synthetic_docx_with_parts(
                r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body><w:p><w:commentRangeStart w:id="7"/><w:r><w:t>Visible text</w:t></w:r><w:commentRangeEnd w:id="7"/></w:p></w:body></w:document>"#,
                Some(&rels_xml),
                None,
                &extra_parts,
            );

            let document = read_docx_bytes(&bytes).expect("unsafe comments target should degrade");

            assert!(document.comments.is_empty());
            assert_all_comment_ids_empty(&document);
            assert!(document
                .warnings
                .iter()
                .any(|warning| warning.code == expected_code));
            assert_docx_comment_warnings_are_generic(&document, "Hidden body");
        }
    }

    #[test]
    fn ignores_malformed_missing_and_unanchored_docx_comment_ranges_without_hidden_metadata() {
        for document_xml in [
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:commentRangeStart w:id="7"/><w:r><w:t>Visible text</w:t></w:r></w:p></w:body></w:document>"#,
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:commentRangeStart w:id="8"/><w:r><w:t>Visible text</w:t></w:r><w:commentRangeEnd w:id="8"/></w:p></w:body></w:document>"#,
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Visible text</w:t></w:r><w:r><w:commentReference w:id="7"/></w:r></w:p></w:body></w:document>"#,
        ] {
            let bytes = synthetic_docx_with_parts(
                document_xml,
                Some(
                    r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rCmt1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="comments.xml"/>
</Relationships>"#,
                ),
                None,
                &[(
                    "word/comments.xml",
                    r#"<w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:comment w:id="7" w:author="Private Host" w:date="2026-06-25T10:00:00Z"><w:p><w:r><w:t>Hidden body</w:t></w:r></w:p></w:comment></w:comments>"#,
                )],
            );

            let document = read_docx_bytes(&bytes).expect("malformed comment range should degrade");

            assert!(document.comments.is_empty());
            assert_all_comment_ids_empty(&document);
            assert!(document.warnings.iter().any(|warning| {
                matches!(
                    warning.code.as_str(),
                    "docx_comment_range_ignored" | "docx_inline_metadata_ignored"
                )
            }));
            assert_docx_comment_warnings_are_generic(&document, "Hidden body");
        }
    }

    #[test]
    fn exports_docx_comments_package_markers_escaped_metadata_and_imports_back() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: ParagraphFormat::default(),
            inlines: vec![Inline {
                text: "Needs review".to_string(),
                marks: Vec::new(),
                link: None,
                comment_ids: vec!["cmt-review".to_string()],
                style: InlineStyle::default(),
                field: None,
                note_reference: None,
                tracked_change: None,
            }],
        })];
        let now = Utc::now();
        document.comments.insert(
            "cmt-review".to_string(),
            CommentThread {
                id: "cmt-review".to_string(),
                author: "A < B & C".to_string(),
                body: "Needs <escape> & review".to_string(),
                created_at: now,
                updated_at: now,
                resolved: true,
            },
        );

        let bytes = write_docx_bytes(&document).expect("docx should write comments");
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
        let mut comments_xml = String::new();
        archive
            .by_name("word/comments.xml")
            .expect("comments part should exist")
            .read_to_string(&mut comments_xml)
            .expect("comments part should read");

        assert!(content_types.contains("/word/comments.xml"));
        assert!(document_rels.contains("relationships/comments"));
        assert!(document_rels.contains(r#"Target="comments.xml""#));
        assert!(document_xml.contains(r#"<w:commentRangeStart w:id="0"/>"#));
        assert!(document_xml.contains(r#"<w:commentRangeEnd w:id="0"/>"#));
        assert!(document_xml.contains(r#"<w:commentReference w:id="0"/>"#));
        assert!(comments_xml.contains(r#"w:author="A &lt; B &amp; C""#));
        assert!(comments_xml.contains("Needs &lt;escape&gt; &amp; review"));
        assert!(!comments_xml.contains("resolved"));

        let parsed = read_docx_bytes(&bytes).expect("written comments package should import");
        assert_eq!(parsed.comments.len(), 1);
        let (comment_id, comment) = parsed
            .comments
            .iter()
            .next()
            .expect("comment should import");
        assert_eq!(comment.author, "A < B & C");
        assert_eq!(comment.body, "Needs <escape> & review");
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("paragraph expected");
        };
        assert_eq!(paragraph.inlines[0].comment_ids, vec![comment_id.clone()]);
    }

    #[test]
    fn exports_one_docx_comment_range_across_split_inline_runs() {
        let mut first = Inline::text("Needs ");
        first.comment_ids = vec!["cmt-review".to_string()];
        let mut second = Inline::text("bold");
        second.marks = vec![InlineMark::Bold];
        second.comment_ids = vec!["cmt-review".to_string()];
        let mut third = Inline::text(" review");
        third.comment_ids = vec!["cmt-review".to_string()];

        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: ParagraphFormat::default(),
            inlines: vec![first, second, third],
        })];
        let now = Utc::now();
        document.comments.insert(
            "cmt-review".to_string(),
            CommentThread {
                id: "cmt-review".to_string(),
                author: "Reviewer".to_string(),
                body: "Review across formatting".to_string(),
                created_at: now,
                updated_at: now,
                resolved: false,
            },
        );

        let bytes = write_docx_bytes(&document).expect("docx should write comments");
        validate_docx_package(&bytes, PackageLimits::default()).expect("written package validates");
        let mut archive =
            ZipArchive::new(Cursor::new(bytes.as_slice())).expect("written docx should open");
        let mut document_xml = String::new();
        archive
            .by_name(DOCUMENT_XML)
            .expect("document xml should exist")
            .read_to_string(&mut document_xml)
            .expect("document xml should read");

        assert_eq!(
            document_xml
                .matches(r#"<w:commentRangeStart w:id="0"/>"#)
                .count(),
            1
        );
        assert_eq!(
            document_xml
                .matches(r#"<w:commentRangeEnd w:id="0"/>"#)
                .count(),
            1
        );
        assert_eq!(
            document_xml
                .matches(r#"<w:commentReference w:id="0"/>"#)
                .count(),
            1
        );

        let parsed = read_docx_bytes(&bytes).expect("written comments package should import");
        assert_eq!(parsed.comments.len(), 1);
        let (comment_id, _) = parsed
            .comments
            .iter()
            .next()
            .expect("comment should import");
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("paragraph expected");
        };
        assert_eq!(paragraph.inlines.len(), 3);
        assert!(paragraph
            .inlines
            .iter()
            .all(|inline| inline.comment_ids == vec![comment_id.clone()]));
    }

    #[test]
    fn imports_simple_docx_footnotes_and_endnotes_from_body_list_and_table() {
        let bytes = synthetic_docx_with_parts(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:p><w:r><w:t>Claim</w:t></w:r><w:r><w:footnoteReference w:id="5"/></w:r></w:p>
  <w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="7"/></w:numPr></w:pPr><w:r><w:t>List item</w:t></w:r><w:r><w:endnoteReference w:id="9"/></w:r></w:p>
  <w:tbl><w:tr><w:tc><w:p><w:r><w:t>Cell</w:t></w:r><w:r><w:footnoteReference w:id="6"/></w:r></w:p></w:tc></w:tr></w:tbl>
</w:body></w:document>"#,
            Some(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rFootnotes" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes" Target="footnotes.xml"/>
<Relationship Id="rEndnotes" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/endnotes" Target="endnotes.xml"/>
</Relationships>"#,
            ),
            Some(
                r#"<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:abstractNum w:abstractNumId="4"><w:lvl w:ilvl="0"><w:numFmt w:val="decimal"/></w:lvl></w:abstractNum>
<w:num w:numId="7"><w:abstractNumId w:val="4"/></w:num>
</w:numbering>"#,
            ),
            &[
                (
                    "word/footnotes.xml",
                    r#"<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:footnote w:type="separator" w:id="-1"/>
<w:footnote w:type="continuationSeparator" w:id="0"/>
<w:footnote w:id="5"><w:p><w:r><w:footnoteRef/></w:r><w:r><w:t>Foot body</w:t></w:r><w:r><w:tab/></w:r><w:r><w:t>tab</w:t></w:r></w:p><w:p><w:r><w:t>Second line</w:t></w:r></w:p></w:footnote>
<w:footnote w:id="6"><w:p><w:r><w:t>Cell footnote</w:t></w:r></w:p></w:footnote>
</w:footnotes>"#,
                ),
                (
                    "word/endnotes.xml",
                    r#"<w:endnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:endnote w:type="separator" w:id="-1"/>
<w:endnote w:type="continuationSeparator" w:id="0"/>
<w:endnote w:id="9"><w:p><w:r><w:endnoteRef/></w:r><w:r><w:t>End body</w:t></w:r><w:r><w:br/></w:r><w:r><w:t>line</w:t></w:r></w:p></w:endnote>
</w:endnotes>"#,
                ),
            ],
        );

        let document = read_docx_bytes(&bytes).expect("docx notes should import");

        assert!(document.warnings.is_empty(), "{:?}", document.warnings);
        assert_eq!(document.notes.len(), 3);
        assert_eq!(
            document.notes["note-docx-footnote-1"].body,
            "Foot body\ttab\nSecond line"
        );
        assert_eq!(
            document.notes["note-docx-footnote-1"].kind,
            NoteKind::Footnote
        );
        assert_eq!(document.notes["note-docx-endnote-1"].body, "End body\nline");
        assert_eq!(
            document.notes["note-docx-endnote-1"].kind,
            NoteKind::Endnote
        );
        assert_eq!(document.notes["note-docx-footnote-2"].body, "Cell footnote");

        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("paragraph expected");
        };
        assert_eq!(
            paragraph.inlines[1].note_reference,
            Some(InlineNoteReference {
                id: "note-docx-footnote-1".to_string(),
                kind: NoteKind::Footnote,
                label: "1".to_string(),
            })
        );

        let Block::List(list) = &document.sections[0].blocks[1] else {
            panic!("list expected");
        };
        let Block::Paragraph(list_paragraph) = &list.items[0].blocks[0] else {
            panic!("list paragraph expected");
        };
        assert_eq!(
            list_paragraph.inlines[1].note_reference,
            Some(InlineNoteReference {
                id: "note-docx-endnote-1".to_string(),
                kind: NoteKind::Endnote,
                label: "1".to_string(),
            })
        );

        let Block::Table(table) = &document.sections[0].blocks[2] else {
            panic!("table expected");
        };
        let Block::Paragraph(cell_paragraph) = &table.rows[0].cells[0].blocks[0] else {
            panic!("cell paragraph expected");
        };
        assert_eq!(
            cell_paragraph.inlines[1].note_reference,
            Some(InlineNoteReference {
                id: "note-docx-footnote-2".to_string(),
                kind: NoteKind::Footnote,
                label: "2".to_string(),
            })
        );
    }

    #[test]
    fn unsafe_missing_and_remote_docx_note_relationships_use_generic_fallbacks() {
        for (target, target_mode, extra_parts, expected_code) in [
            (
                "../private/footnotes.xml",
                "",
                Vec::new(),
                "docx_notes_relationship_ignored",
            ),
            (
                "notes/footnotes.xml",
                "",
                Vec::new(),
                "docx_notes_relationship_ignored",
            ),
            (
                "footnotes.xml",
                r#" TargetMode="External""#,
                Vec::new(),
                "docx_notes_relationship_ignored",
            ),
            ("footnotes.xml", "", Vec::new(), "docx_notes_part_missing"),
        ] {
            let rels_xml = format!(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rFootnotes" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes" Target="{target}"{target_mode}/>
</Relationships>"#
            );
            let bytes = synthetic_docx_with_parts(
                r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body><w:p><w:r><w:t>Claim</w:t></w:r><w:r><w:footnoteReference w:id="7"/></w:r></w:p></w:body></w:document>"#,
                Some(&rels_xml),
                None,
                &extra_parts,
            );

            let document = read_docx_bytes(&bytes).expect("unsafe notes should degrade");

            assert!(document.notes.is_empty());
            let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
                panic!("paragraph expected");
            };
            assert_eq!(inline_text(&paragraph.inlines), "Claim[footnote]");
            assert!(paragraph
                .inlines
                .iter()
                .all(|inline| inline.note_reference.is_none()));
            assert!(document
                .warnings
                .iter()
                .any(|warning| warning.code == expected_code));
            assert_docx_note_warnings_are_generic(&document, "Hidden note body");
        }
    }

    #[test]
    fn malformed_unanchored_and_hidden_docx_notes_are_generic_and_not_imported() {
        let bytes = synthetic_docx_with_parts(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body><w:p><w:r><w:t>Claim</w:t></w:r><w:r><w:footnoteReference w:id="7"/></w:r></w:p></w:body></w:document>"#,
            Some(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rFootnotes" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes" Target="footnotes.xml"/>
</Relationships>"#,
            ),
            None,
            &[(
                "word/footnotes.xml",
                r#"<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:footnote w:id="7"><w:p><w:r><w:t>Visible fragment</w:t></w:r><w:ins><w:r><w:t>Hidden note body</w:t></w:r></w:ins></w:p></w:footnote>
<w:footnote w:id="8"><w:p><w:r><w:t>Unanchored private body</w:t></w:r></w:p></w:footnote>
</w:footnotes>"#,
            )],
        );

        let document = read_docx_bytes(&bytes).expect("malformed notes should degrade");

        assert!(document.notes.is_empty());
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("paragraph expected");
        };
        assert_eq!(inline_text(&paragraph.inlines), "Claim[footnote]");
        assert!(paragraph
            .inlines
            .iter()
            .all(|inline| inline.note_reference.is_none()));
        assert!(document
            .warnings
            .iter()
            .any(|warning| warning.code == "docx_note_content_degraded"));
        assert!(document
            .warnings
            .iter()
            .any(|warning| warning.code == "docx_note_reference_ignored"));
        assert!(document
            .warnings
            .iter()
            .any(|warning| warning.code == "docx_unanchored_notes_ignored"));
        assert_docx_note_warnings_are_generic(&document, "Hidden note body");
        assert_docx_note_warnings_are_generic(&document, "Visible fragment");
        assert_docx_note_warnings_are_generic(&document, "Unanchored private body");
    }

    #[test]
    fn excess_docx_notes_import_until_limit_then_use_visible_fallback() {
        let mut body = String::new();
        let mut footnotes = String::from(
            r#"<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
        );
        for index in 1..=MAX_NOTES + 1 {
            body.push_str("<w:r><w:footnoteReference w:id=\"");
            body.push_str(&index.to_string());
            body.push_str("\"/></w:r>");
            footnotes.push_str("<w:footnote w:id=\"");
            footnotes.push_str(&index.to_string());
            footnotes.push_str("\"><w:p><w:r><w:t>Note ");
            footnotes.push_str(&index.to_string());
            footnotes.push_str("</w:t></w:r></w:p></w:footnote>");
        }
        footnotes.push_str("</w:footnotes>");
        let document_xml = format!(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p>{body}</w:p></w:body></w:document>"#
        );
        let bytes = synthetic_docx_with_parts(
            &document_xml,
            Some(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rFootnotes" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes" Target="footnotes.xml"/>
</Relationships>"#,
            ),
            None,
            &[("word/footnotes.xml", &footnotes)],
        );

        let document = read_docx_bytes(&bytes).expect("excess notes should degrade");

        assert_eq!(document.notes.len(), MAX_NOTES);
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("paragraph expected");
        };
        assert_eq!(paragraph.inlines.len(), MAX_NOTES + 1);
        assert!(paragraph.inlines[..MAX_NOTES]
            .iter()
            .all(|inline| inline.note_reference.is_some()));
        assert_eq!(paragraph.inlines[MAX_NOTES].text, "[footnote]");
        assert!(paragraph.inlines[MAX_NOTES].note_reference.is_none());
        assert!(document
            .warnings
            .iter()
            .any(|warning| warning.code == "docx_notes_over_limit"));
        assert_docx_note_warnings_are_generic(&document, "Note 513");
    }

    #[test]
    fn exports_docx_note_parts_references_and_round_trips_without_local_metadata() {
        let mut document = Document::new_untitled();
        document.notes.insert(
            "note-source".to_string(),
            Note {
                id: "note-source".to_string(),
                kind: NoteKind::Footnote,
                body: "Source <body> & details".to_string(),
            },
        );
        document.notes.insert(
            "note-appendix".to_string(),
            Note {
                id: "note-appendix".to_string(),
                kind: NoteKind::Endnote,
                body: "Appendix\tbody".to_string(),
            },
        );
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: ParagraphFormat::default(),
            inlines: vec![
                Inline::text("Claim"),
                Inline::note_reference(InlineNoteReference {
                    id: "note-source".to_string(),
                    kind: NoteKind::Footnote,
                    label: "1".to_string(),
                }),
                Inline::text(" Appendix"),
                Inline::note_reference(InlineNoteReference {
                    id: "note-appendix".to_string(),
                    kind: NoteKind::Endnote,
                    label: "1".to_string(),
                }),
            ],
        })];

        let bytes = write_docx_bytes(&document).expect("docx should write notes");
        validate_docx_package(&bytes, PackageLimits::default()).expect("written package validates");
        let content_types = read_zip_text_part(&bytes, "[Content_Types].xml");
        let document_rels = read_zip_text_part(&bytes, DOCUMENT_RELS);
        let document_xml = read_zip_text_part(&bytes, DOCUMENT_XML);
        let footnotes_xml = read_zip_text_part(&bytes, "word/footnotes.xml");
        let endnotes_xml = read_zip_text_part(&bytes, "word/endnotes.xml");

        assert!(content_types.contains("/word/footnotes.xml"));
        assert!(content_types.contains("/word/endnotes.xml"));
        assert!(document_rels.contains("relationships/footnotes"));
        assert!(document_rels.contains("relationships/endnotes"));
        assert!(document_rels.contains(r#"Target="footnotes.xml""#));
        assert!(document_rels.contains(r#"Target="endnotes.xml""#));
        assert!(document_xml.contains(r#"<w:footnoteReference w:id="1"/>"#));
        assert!(document_xml.contains(r#"<w:endnoteReference w:id="1"/>"#));
        assert!(footnotes_xml.contains("Source &lt;body&gt; &amp; details"));
        assert!(endnotes_xml.contains("<w:tab/>"));
        for xml in [&document_xml, &document_rels, &footnotes_xml, &endnotes_xml] {
            assert!(!xml.contains("note-source"));
            assert!(!xml.contains("note-appendix"));
            assert!(!xml.contains("Users"));
            assert!(!xml.contains("C:/"));
        }

        let parsed = read_docx_bytes(&bytes).expect("written notes package should import");
        assert_eq!(parsed.notes.len(), 2);
        assert_eq!(
            parsed.notes["note-docx-footnote-1"].body,
            "Source <body> & details"
        );
        assert_eq!(parsed.notes["note-docx-endnote-1"].body, "Appendix\tbody");
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("paragraph expected");
        };
        assert_eq!(
            paragraph.inlines[1]
                .note_reference
                .as_ref()
                .map(|reference| (reference.id.as_str(), reference.kind)),
            Some(("note-docx-footnote-1", NoteKind::Footnote))
        );
        assert_eq!(
            paragraph.inlines[3]
                .note_reference
                .as_ref()
                .map(|reference| (reference.id.as_str(), reference.kind)),
            Some(("note-docx-endnote-1", NoteKind::Endnote))
        );
    }

    #[test]
    fn imports_simple_docx_insertions_and_deletions_as_tracked_changes() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body><w:p><w:r><w:t>Before </w:t></w:r><w:ins w:id="999/private" w:author="Reviewer" w:date="2026-06-25T10:00:00Z"><w:r><w:t>new</w:t></w:r></w:ins><w:r><w:t> </w:t></w:r><w:del w:id="1000" w:author="Editor" w:date="2026-06-25T11:00:00Z"><w:r><w:delText>old</w:delText></w:r></w:del></w:p></w:body></w:document>"#,
            None,
            None,
        );

        let document = read_docx_bytes(&bytes).expect("tracked revisions should import");
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("paragraph expected");
        };

        assert_eq!(paragraph.inlines.len(), 4);
        assert_eq!(paragraph.inlines[1].text, "new");
        let insertion = paragraph.inlines[1]
            .tracked_change
            .as_ref()
            .expect("inserted text should be tracked");
        assert_eq!(insertion.id, "chg-docx-change-1");
        assert_eq!(insertion.kind, TrackedChangeKind::Insertion);
        assert_eq!(insertion.author, "Reviewer");
        assert_eq!(
            insertion.created_at,
            DateTime::parse_from_rfc3339("2026-06-25T10:00:00Z")
                .expect("date should parse")
                .with_timezone(&Utc)
        );

        assert_eq!(paragraph.inlines[3].text, "old");
        let deletion = paragraph.inlines[3]
            .tracked_change
            .as_ref()
            .expect("deleted text should be tracked");
        assert_eq!(deletion.id, "chg-docx-change-2");
        assert_eq!(deletion.kind, TrackedChangeKind::Deletion);
        assert_eq!(deletion.author, "Editor");
    }

    #[test]
    fn imports_docx_revisions_inside_lists_and_table_cells() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="7"/></w:numPr></w:pPr><w:ins w:author="Reviewer"><w:r><w:t>List item</w:t></w:r></w:ins></w:p>
  <w:tbl><w:tr><w:tc><w:p><w:del w:author="Reviewer"><w:r><w:delText>Cell text</w:delText></w:r></w:del></w:p></w:tc></w:tr></w:tbl>
</w:body></w:document>"#,
            None,
            Some(
                r#"<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:abstractNum w:abstractNumId="4"><w:lvl w:ilvl="0"><w:numFmt w:val="bullet"/></w:lvl></w:abstractNum>
<w:num w:numId="7"><w:abstractNumId w:val="4"/></w:num>
</w:numbering>"#,
            ),
        );

        let document = read_docx_bytes(&bytes).expect("nested tracked revisions should import");

        let Block::List(list) = &document.sections[0].blocks[0] else {
            panic!("list expected");
        };
        let Block::Paragraph(list_paragraph) = &list.items[0].blocks[0] else {
            panic!("list paragraph expected");
        };
        assert_eq!(
            list_paragraph.inlines[0]
                .tracked_change
                .as_ref()
                .map(|change| change.kind),
            Some(TrackedChangeKind::Insertion)
        );

        let Block::Table(table) = &document.sections[0].blocks[1] else {
            panic!("table expected");
        };
        let Block::Paragraph(cell_paragraph) = &table.rows[0].cells[0].blocks[0] else {
            panic!("cell paragraph expected");
        };
        assert_eq!(
            cell_paragraph.inlines[0]
                .tracked_change
                .as_ref()
                .map(|change| change.kind),
            Some(TrackedChangeKind::Deletion)
        );
    }

    #[test]
    fn imports_nested_and_unsupported_docx_revisions_as_visible_fallback_text() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body><w:p><w:ins w:author="Reviewer"><w:r><w:t>Outer </w:t></w:r><w:del w:author="Reviewer"><w:r><w:delText>nested</w:delText></w:r></w:del></w:ins><w:moveFrom><w:r><w:t> moved</w:t></w:r></w:moveFrom></w:p></w:body></w:document>"#,
            None,
            None,
        );

        let document = read_docx_bytes(&bytes).expect("unsupported revisions should degrade");
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("paragraph expected");
        };

        assert_eq!(paragraph.inlines[0].text, "Outer ");
        assert_eq!(
            paragraph.inlines[0]
                .tracked_change
                .as_ref()
                .map(|change| change.kind),
            Some(TrackedChangeKind::Insertion)
        );
        assert_eq!(paragraph.inlines[1].text, "nested");
        assert!(paragraph.inlines[1].tracked_change.is_none());
        assert_eq!(paragraph.inlines[2].text, " moved");
        assert!(paragraph.inlines[2].tracked_change.is_none());
        assert!(document
            .warnings
            .iter()
            .any(|warning| warning.code == "docx_revision_markup_degraded"));
        assert_docx_revision_warnings_are_generic(&document);
    }

    #[test]
    fn imports_docx_revision_authors_and_dates_with_privacy_safe_fallbacks() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body><w:p><w:ins w:id="C:/placeholder/raw-id" w:author="C:/placeholder/reviewer" w:date="not-a-date"><w:r><w:t>private author</w:t></w:r></w:ins></w:p></w:body></w:document>"#,
            None,
            None,
        );

        let document = read_docx_bytes(&bytes).expect("unsafe revision metadata should degrade");
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("paragraph expected");
        };
        let change = paragraph.inlines[0]
            .tracked_change
            .as_ref()
            .expect("revision should still import");

        assert_eq!(change.id, "chg-docx-change-1");
        assert_eq!(change.author, IMPORTED_DOCX_REVISION_AUTHOR);
        assert_eq!(change.created_at, epoch_utc());
        assert!(!change.id.contains("Users"));
        assert!(document
            .warnings
            .iter()
            .any(|warning| warning.code == "docx_revision_metadata_degraded"));
        assert_docx_revision_warnings_are_generic(&document);
    }

    #[test]
    fn imports_excess_docx_revisions_as_visible_untracked_text() {
        let mut body = String::new();
        for index in 0..=MAX_DOCX_REVISIONS {
            body.push_str("<w:ins w:author=\"Reviewer\"><w:r><w:t>");
            body.push_str(&format!("change-{index}"));
            body.push_str("</w:t></w:r></w:ins>");
        }
        let document_xml = format!(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p>{body}</w:p></w:body></w:document>"#
        );
        let bytes = synthetic_docx(&document_xml, None, None);

        let document = read_docx_bytes(&bytes).expect("over-limit revisions should degrade");
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("paragraph expected");
        };

        assert_eq!(paragraph.inlines.len(), MAX_DOCX_REVISIONS + 1);
        assert!(paragraph.inlines[..MAX_DOCX_REVISIONS]
            .iter()
            .all(|inline| inline.tracked_change.is_some()));
        let fallback = &paragraph.inlines[MAX_DOCX_REVISIONS];
        assert_eq!(fallback.text, format!("change-{MAX_DOCX_REVISIONS}"));
        assert!(fallback.tracked_change.is_none());
        assert!(document
            .warnings
            .iter()
            .any(|warning| warning.code == "docx_revisions_over_limit"));
        assert_docx_revision_warnings_are_generic(&document);
    }

    #[test]
    fn exports_docx_tracked_insertions_and_deletions_with_revision_markup() {
        let created_at = DateTime::parse_from_rfc3339("2026-06-25T10:00:00Z")
            .expect("date should parse")
            .with_timezone(&Utc);
        let mut inserted = Inline::text("new");
        inserted.tracked_change = Some(TrackedChange {
            id: "chg-insert".to_string(),
            kind: TrackedChangeKind::Insertion,
            author: "Reviewer".to_string(),
            created_at,
        });
        let mut deleted = Inline::text("old");
        deleted.tracked_change = Some(TrackedChange {
            id: "chg-delete".to_string(),
            kind: TrackedChangeKind::Deletion,
            author: "Editor".to_string(),
            created_at,
        });
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: ParagraphFormat::default(),
            inlines: vec![
                Inline::text("Before "),
                inserted,
                Inline::text(" "),
                deleted,
            ],
        })];

        let bytes = write_docx_bytes(&document).expect("docx should write tracked revisions");
        validate_docx_package(&bytes, PackageLimits::default()).expect("written package validates");
        let document_xml = read_zip_text_part(&bytes, DOCUMENT_XML);

        assert!(document_xml
            .contains(r#"<w:ins w:id="0" w:author="Reviewer" w:date="2026-06-25T10:00:00Z">"#));
        assert!(document_xml
            .contains(r#"<w:del w:id="1" w:author="Editor" w:date="2026-06-25T10:00:00Z">"#));
        assert!(document_xml.contains(r#"<w:t xml:space="preserve">new</w:t>"#));
        assert!(document_xml.contains(r#"<w:delText xml:space="preserve">old</w:delText>"#));

        let parsed = read_docx_bytes(&bytes).expect("written tracked revisions should import");
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("paragraph expected");
        };
        assert_eq!(
            paragraph.inlines[1]
                .tracked_change
                .as_ref()
                .map(|change| change.kind),
            Some(TrackedChangeKind::Insertion)
        );
        assert_eq!(
            paragraph.inlines[3]
                .tracked_change
                .as_ref()
                .map(|change| change.kind),
            Some(TrackedChangeKind::Deletion)
        );
    }

    #[test]
    fn skips_docx_comment_export_for_inline_that_is_also_tracked_change() {
        let created_at = DateTime::parse_from_rfc3339("2026-06-25T10:00:00Z")
            .expect("date should parse")
            .with_timezone(&Utc);
        let mut inline = Inline::text("commented change");
        inline.comment_ids = vec!["cmt-review".to_string()];
        inline.tracked_change = Some(TrackedChange {
            id: "chg-private-author".to_string(),
            kind: TrackedChangeKind::Insertion,
            author: "C:/placeholder/local-author".to_string(),
            created_at,
        });
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: ParagraphFormat::default(),
            inlines: vec![inline],
        })];
        document.comments.insert(
            "cmt-review".to_string(),
            CommentThread {
                id: "cmt-review".to_string(),
                author: "Reviewer".to_string(),
                body: "Review note".to_string(),
                created_at,
                updated_at: created_at,
                resolved: false,
            },
        );

        let bytes = write_docx_bytes(&document)
            .expect("docx should write tracked revision without comment export");
        let document_xml = read_zip_text_part(&bytes, DOCUMENT_XML);
        let mut archive =
            ZipArchive::new(Cursor::new(bytes.as_slice())).expect("written docx should open");

        assert!(document_xml.contains(r#"<w:ins w:id="0" w:author="Local User""#));
        assert!(!document_xml.contains("C:/placeholder"));
        assert!(!document_xml.contains("commentRangeStart"));
        assert!(matches!(
            archive.by_name("word/comments.xml"),
            Err(zip::result::ZipError::FileNotFound)
        ));
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
                    r#"<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p><w:r><w:t>Page </w:t></w:r><w:fldSimple w:instr=" PAGE "><w:r><w:rPr><w:b/><w:sz w:val="28"/><w:color w:val="1F2937"/><w:highlight w:val="yellow"/></w:rPr><w:t>1</w:t></w:r></w:fldSimple></w:p></w:hdr>"#,
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
        assert_eq!(header.inlines[1].marks, vec![InlineMark::Bold]);
        assert_eq!(header.inlines[1].style.font_size_pt, Some(14));
        assert_eq!(
            header.inlines[1].style.text_color.as_deref(),
            Some("#1f2937")
        );
        assert_eq!(
            header.inlines[1].style.highlight_color.as_deref(),
            Some("#fff3bf")
        );
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
    fn exports_and_imports_docx_field_inline_formatting() {
        let mut body_field = Inline::field(PageField::PageNumber);
        body_field.marks = vec![InlineMark::Bold, InlineMark::Strikethrough];
        body_field.style = InlineStyle {
            font_family: None,
            font_size_pt: Some(14),
            text_color: Some("#1f2937".to_string()),
            highlight_color: Some("#fff3bf".to_string()),
        };
        let mut header_field = Inline::field(PageField::Date);
        header_field.marks = vec![InlineMark::Italic, InlineMark::Superscript];
        header_field.style = InlineStyle {
            font_family: None,
            font_size_pt: Some(9),
            text_color: Some("#0066cc".to_string()),
            highlight_color: Some("#dbeafe".to_string()),
        };

        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: ParagraphFormat::default(),
            inlines: vec![Inline::text("Page "), body_field],
        })];
        document.sections[0].page_regions.header.blocks =
            vec![PageRegionBlock::Paragraph(PageRegionParagraph {
                inlines: vec![Inline::text("Updated "), header_field],
            })];

        let bytes = write_docx_bytes(&document).expect("docx should write styled fields");
        let document_xml = read_zip_text_part(&bytes, DOCUMENT_XML);
        let header_xml = read_zip_text_part(&bytes, "word/header1.xml");

        assert!(document_xml.contains(r#"<w:fldSimple w:instr=" PAGE ">"#));
        assert!(document_xml.contains("<w:b/>"));
        assert!(document_xml.contains("<w:strike/>"));
        assert!(document_xml.contains(r#"<w:sz w:val="28"/>"#));
        assert!(document_xml.contains(r#"<w:color w:val="1F2937"/>"#));
        assert!(document_xml.contains(r#"<w:highlight w:val="yellow"/>"#));
        assert!(header_xml.contains(r#"<w:fldSimple w:instr=" DATE ">"#));
        assert!(header_xml.contains("<w:i/>"));
        assert!(header_xml.contains(r#"<w:vertAlign w:val="superscript"/>"#));
        assert!(header_xml.contains(r#"<w:sz w:val="18"/>"#));
        assert!(header_xml.contains(r#"<w:color w:val="0066CC"/>"#));
        assert!(header_xml.contains(r#"<w:highlight w:val="cyan"/>"#));

        let parsed = read_docx_bytes(&bytes).expect("styled fields should import");
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("body paragraph should import");
        };
        assert_eq!(paragraph.inlines[1].field, Some(PageField::PageNumber));
        assert_eq!(
            paragraph.inlines[1].marks,
            vec![InlineMark::Bold, InlineMark::Strikethrough]
        );
        assert_eq!(paragraph.inlines[1].style.font_size_pt, Some(14));
        assert_eq!(
            paragraph.inlines[1].style.text_color.as_deref(),
            Some("#1f2937")
        );
        assert_eq!(
            paragraph.inlines[1].style.highlight_color.as_deref(),
            Some("#fff3bf")
        );

        let PageRegionBlock::Paragraph(header) = &parsed.sections[0].page_regions.header.blocks[0];
        assert_eq!(header.inlines[1].field, Some(PageField::Date));
        assert_eq!(
            header.inlines[1].marks,
            vec![InlineMark::Italic, InlineMark::Superscript]
        );
        assert_eq!(header.inlines[1].style.font_size_pt, Some(9));
        assert_eq!(
            header.inlines[1].style.text_color.as_deref(),
            Some("#0066cc")
        );
        assert_eq!(
            header.inlines[1].style.highlight_color.as_deref(),
            Some("#dbeafe")
        );
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
                column_widths: Vec::new(),
                rows: vec![TableRow {
                    cells: vec![TableCell {
                        presentation: Default::default(),
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
    fn exports_and_imports_docx_inline_formatting() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: ParagraphFormat::default(),
            inlines: vec![
                Inline {
                    text: "Styled".to_string(),
                    marks: vec![
                        InlineMark::Bold,
                        InlineMark::Italic,
                        InlineMark::Underline,
                        InlineMark::Strikethrough,
                        InlineMark::Superscript,
                    ],
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
                },
                Inline {
                    text: " small".to_string(),
                    marks: vec![InlineMark::Subscript],
                    link: None,
                    comment_ids: Vec::new(),
                    style: InlineStyle {
                        font_family: None,
                        font_size_pt: Some(9),
                        text_color: Some("#0066cc".to_string()),
                        highlight_color: Some("#dbeafe".to_string()),
                    },
                    field: None,
                    note_reference: None,
                    tracked_change: None,
                },
            ],
        })];

        let bytes = write_docx_bytes(&document).expect("docx should write formatted inline text");
        validate_docx_package(&bytes, PackageLimits::default()).expect("written package validates");
        let document_xml = read_zip_text_part(&bytes, DOCUMENT_XML);

        assert!(document_xml.contains("<w:b/>"));
        assert!(document_xml.contains("<w:i/>"));
        assert!(document_xml.contains(r#"<w:u w:val="single"/>"#));
        assert!(document_xml.contains("<w:strike/>"));
        assert!(document_xml.contains(r#"<w:vertAlign w:val="superscript"/>"#));
        assert!(document_xml.contains(r#"<w:vertAlign w:val="subscript"/>"#));
        assert!(document_xml.contains(r#"<w:sz w:val="28"/>"#));
        assert!(document_xml.contains(r#"<w:sz w:val="18"/>"#));
        assert!(document_xml.contains(r#"<w:color w:val="1F2937"/>"#));
        assert!(document_xml.contains(r#"<w:color w:val="0066CC"/>"#));
        assert!(document_xml.contains(r#"<w:highlight w:val="yellow"/>"#));
        assert!(document_xml.contains(r#"<w:highlight w:val="cyan"/>"#));
        assert!(!document_xml.contains("w:rFonts"));

        let parsed = read_docx_bytes(&bytes).expect("formatted inline text should import");
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("formatted inline text should round-trip through docx converter");
        };

        assert_eq!(
            paragraph.inlines[0].marks,
            vec![
                InlineMark::Bold,
                InlineMark::Italic,
                InlineMark::Underline,
                InlineMark::Strikethrough,
                InlineMark::Superscript,
            ]
        );
        assert_eq!(paragraph.inlines[0].style.font_family, None);
        assert_eq!(paragraph.inlines[0].style.font_size_pt, Some(14));
        assert_eq!(
            paragraph.inlines[0].style.text_color.as_deref(),
            Some("#1f2937")
        );
        assert_eq!(
            paragraph.inlines[0].style.highlight_color.as_deref(),
            Some("#fff3bf")
        );
        assert_eq!(paragraph.inlines[1].marks, vec![InlineMark::Subscript]);
        assert_eq!(paragraph.inlines[1].style.font_size_pt, Some(9));
        assert_eq!(
            paragraph.inlines[1].style.text_color.as_deref(),
            Some("#0066cc")
        );
        assert_eq!(
            paragraph.inlines[1].style.highlight_color.as_deref(),
            Some("#dbeafe")
        );
    }

    #[test]
    fn exports_and_imports_docx_paragraph_formatting() {
        let mut document = Document::new_untitled();
        let expected_format = ParagraphFormat {
            alignment: Some(ParagraphAlignment::Center),
            line_spacing_per_mille: Some(1500),
            spacing_before_mm: Some(3),
            spacing_after_mm: Some(5),
            indent_start_mm: Some(8),
            indent_end_mm: Some(4),
            first_line_indent_mm: Some(3),
        };
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: expected_format.clone(),
            inlines: vec![Inline::text("Formatted export")],
        })];

        let bytes = write_docx_bytes(&document).expect("docx should write formatted paragraph");
        validate_docx_package(&bytes, PackageLimits::default()).expect("written package validates");
        let document_xml = read_zip_text_part(&bytes, DOCUMENT_XML);

        assert!(document_xml.contains(
            r#"<w:spacing w:line="360" w:lineRule="auto" w:before="170" w:after="283"/>"#
        ));
        assert!(document_xml.contains(r#"<w:ind w:left="454" w:right="227" w:firstLine="170"/>"#));
        assert!(document_xml.contains(r#"<w:jc w:val="center"/>"#));

        let parsed = read_docx_bytes(&bytes).expect("formatted paragraph should import");
        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("formatted paragraph should round-trip through docx converter");
        };
        assert_eq!(paragraph.format, expected_format);
    }

    #[test]
    fn exports_and_imports_docx_page_setup() {
        let mut document = Document::new_untitled();
        let expected_page = PageSetup {
            width_mm: 216,
            height_mm: 279,
            margin_top_mm: 20,
            margin_right_mm: 15,
            margin_bottom_mm: 25,
            margin_left_mm: 18,
        };
        document.sections[0].page = expected_page.clone();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: ParagraphFormat::default(),
            inlines: vec![Inline::text("Page setup export")],
        })];

        let bytes = write_docx_bytes(&document).expect("docx should write page setup");
        validate_docx_package(&bytes, PackageLimits::default()).expect("written package validates");
        let document_xml = read_zip_text_part(&bytes, DOCUMENT_XML);

        assert!(document_xml.contains(&format!(
            r#"<w:pgSz w:w="{}" w:h="{}"/>"#,
            mm_to_docx_twips(expected_page.width_mm),
            mm_to_docx_twips(expected_page.height_mm)
        )));
        assert!(document_xml.contains(&format!(
            r#"<w:pgMar w:top="{}" w:right="{}" w:bottom="{}" w:left="{}" w:header="720" w:footer="720" w:gutter="0"/>"#,
            mm_to_docx_twips(expected_page.margin_top_mm),
            mm_to_docx_twips(expected_page.margin_right_mm),
            mm_to_docx_twips(expected_page.margin_bottom_mm),
            mm_to_docx_twips(expected_page.margin_left_mm)
        )));

        let parsed = read_docx_bytes(&bytes).expect("written page setup should import");
        assert_eq!(parsed.sections[0].page, expected_page);
    }

    #[test]
    fn exports_and_imports_generated_docx_toc_with_bookmark_targets() {
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
                        text: "Deep details".to_string(),
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
                inlines: vec![Inline::text("Deep details")],
            }),
        ];

        let bytes = write_docx_bytes(&document).expect("docx should write toc");
        validate_docx_package(&bytes, PackageLimits::default()).expect("written package validates");
        let document_xml = read_zip_text_part(&bytes, DOCUMENT_XML);
        let styles_xml = read_zip_text_part(&bytes, "word/styles.xml");

        assert!(document_xml.contains(r#"<w:pStyle w:val="Word900TocTitle"/>"#));
        assert!(document_xml.contains(r#"<w:pStyle w:val="Word900TocEntry1"/>"#));
        assert!(document_xml.contains(r#"<w:pStyle w:val="Word900TocEntry3"/>"#));
        assert!(document_xml.contains(r#"<w:hyperlink w:anchor="bm-intro">"#));
        assert!(document_xml.contains(r#"<w:hyperlink w:anchor="bm-details">"#));
        assert!(document_xml.contains(r#"w:name="bm-intro""#));
        assert!(document_xml.contains(r#"w:name="bm-details""#));
        assert!(styles_xml.contains("Word900TocEntry3"));
        assert!(!document_xml.contains("word900:"));

        let parsed = read_docx_bytes(&bytes).expect("written toc package should import");
        let Block::TableOfContents(table_of_contents) = &parsed.sections[0].blocks[0] else {
            panic!("toc should round-trip through docx converter");
        };
        assert_eq!(table_of_contents.title, "Contents");
        assert_eq!(table_of_contents.entries.len(), 2);
        assert_eq!(table_of_contents.entries[0].level, 1);
        assert_eq!(table_of_contents.entries[0].text, "Intro");
        assert_eq!(table_of_contents.entries[0].target_bookmark_id, "bm-intro");
        assert_eq!(table_of_contents.entries[1].level, 3);
        assert_eq!(
            table_of_contents.entries[1].target_bookmark_id,
            "bm-details"
        );

        let Block::Heading(intro) = &parsed.sections[0].blocks[1] else {
            panic!("intro heading should round-trip through docx converter");
        };
        assert_eq!(intro.bookmark_id.as_deref(), Some("bm-intro"));
    }

    #[test]
    fn imports_generated_toc_style_without_internal_link_as_visible_paragraph() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:p><w:pPr><w:pStyle w:val="Word900TocEntry1"/></w:pPr><w:r><w:t>Unlinked entry</w:t></w:r></w:p>
</w:body></w:document>"#,
            None,
            None,
        );

        let parsed = read_docx_bytes(&bytes).expect("visible fallback should import");

        let Block::Paragraph(paragraph) = &parsed.sections[0].blocks[0] else {
            panic!("unlinked toc row should remain visible paragraph");
        };
        assert_eq!(inline_text(&paragraph.inlines), "Unlinked entry");
    }

    #[test]
    fn omits_generated_toc_hyperlinks_for_duplicate_docx_bookmark_targets() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::TableOfContents(TableOfContents {
                title: "Contents".to_string(),
                entries: vec![TableOfContentsEntry {
                    level: 1,
                    text: "Duplicate".to_string(),
                    target_bookmark_id: "bm-duplicate".to_string(),
                }],
            }),
            Block::Heading(Heading {
                bookmark_id: Some("bm-duplicate".to_string()),
                level: 1,
                inlines: vec![Inline::text("First")],
            }),
            Block::Heading(Heading {
                bookmark_id: Some("bm-duplicate".to_string()),
                level: 1,
                inlines: vec![Inline::text("Second")],
            }),
        ];

        let bytes = write_docx_bytes(&document).expect("docx should write duplicate bookmarks");
        let document_xml = read_zip_text_part(&bytes, DOCUMENT_XML);

        assert!(document_xml.contains("Duplicate"));
        assert!(!document_xml.contains(r#"<w:hyperlink w:anchor="bm-duplicate">"#));
        assert!(!document_xml.contains(r#"w:name="bm-duplicate""#));
    }

    #[test]
    fn exports_and_imports_table_column_width_grid_hints() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Table(Table {
            column_widths: vec![250, 750],
            rows: vec![TableRow {
                cells: vec![table_cell_with_text("A1"), table_cell_with_text("B1")],
            }],
        })];

        let bytes = write_docx_bytes(&document).expect("docx should write table widths");
        validate_docx_package(&bytes, PackageLimits::default()).expect("written package validates");
        let document_xml = read_zip_text_part(&bytes, DOCUMENT_XML);

        assert!(document_xml
            .contains(r#"<w:tblGrid><w:gridCol w:w="2500"/><w:gridCol w:w="7500"/></w:tblGrid>"#));
        assert!(!document_xml.contains("word900:column-widths"));

        let parsed = read_docx_bytes(&bytes).expect("written package should import");
        let Block::Table(table) = &parsed.sections[0].blocks[0] else {
            panic!("table should round-trip through docx converter");
        };
        assert_eq!(table.column_widths, vec![250, 750]);
    }

    #[test]
    fn exports_and_imports_table_cell_presentation() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Table(Table {
            column_widths: Vec::new(),
            rows: vec![TableRow {
                cells: vec![TableCell {
                    presentation: TableCellPresentation {
                        background_color: Some("#dbeafe".to_string()),
                        text_alignment: Some(ParagraphAlignment::Center),
                        border: TableCellBorder::Hidden,
                    },
                    blocks: vec![Block::Paragraph(Paragraph {
                        bookmark_id: None,
                        style: StyleId::from("body"),
                        format: ParagraphFormat::default(),
                        inlines: vec![Inline::text("Styled")],
                    })],
                }],
            }],
        })];

        let bytes = write_docx_bytes(&document).expect("docx should write styled cell");
        validate_docx_package(&bytes, PackageLimits::default()).expect("written package validates");
        let document_xml = read_zip_text_part(&bytes, DOCUMENT_XML);

        assert!(document_xml.contains(r#"<w:shd w:val="clear" w:fill="DBEAFE"/>"#));
        assert!(document_xml.contains(r#"<w:top w:val="nil"/>"#));
        assert!(document_xml.contains(r#"<w:left w:val="nil"/>"#));
        assert!(document_xml.contains(r#"<w:bottom w:val="nil"/>"#));
        assert!(document_xml.contains(r#"<w:right w:val="nil"/>"#));
        assert!(document_xml.contains(r#"<w:jc w:val="center"/>"#));

        let parsed = read_docx_bytes(&bytes).expect("written styled cell should import");
        let Block::Table(table) = &parsed.sections[0].blocks[0] else {
            panic!("table should round-trip through docx converter");
        };
        let cell = &table.rows[0].cells[0];
        assert_eq!(
            cell.presentation,
            TableCellPresentation {
                background_color: Some("#dbeafe".to_string()),
                text_alignment: Some(ParagraphAlignment::Center),
                border: TableCellBorder::Hidden,
            }
        );
        let Block::Paragraph(paragraph) = &cell.blocks[0] else {
            panic!("cell paragraph should remain editable");
        };
        assert_eq!(paragraph.format.alignment, None);
    }

    #[test]
    fn exports_and_imports_heading_table_cell_alignment() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Table(Table {
            column_widths: Vec::new(),
            rows: vec![TableRow {
                cells: vec![TableCell {
                    presentation: TableCellPresentation {
                        background_color: None,
                        text_alignment: Some(ParagraphAlignment::Right),
                        border: TableCellBorder::Visible,
                    },
                    blocks: vec![Block::Heading(Heading {
                        bookmark_id: None,
                        level: 2,
                        inlines: vec![Inline::text("Heading cell")],
                    })],
                }],
            }],
        })];

        let bytes = write_docx_bytes(&document).expect("docx should write heading cell");
        let document_xml = read_zip_text_part(&bytes, DOCUMENT_XML);

        assert!(document_xml.contains(r#"<w:pStyle w:val="Heading2"/><w:jc w:val="right"/>"#));

        let parsed = read_docx_bytes(&bytes).expect("written heading cell should import");
        let Block::Table(table) = &parsed.sections[0].blocks[0] else {
            panic!("table should round-trip through docx converter");
        };
        let cell = &table.rows[0].cells[0];
        assert_eq!(
            cell.presentation.text_alignment,
            Some(ParagraphAlignment::Right)
        );
        let Block::Heading(heading) = &cell.blocks[0] else {
            panic!("cell heading should remain editable");
        };
        assert_eq!(inline_text(&heading.inlines), "Heading cell");
    }

    #[test]
    fn imports_docx_table_cell_presentation_from_safe_subset() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:tbl>
    <w:tr>
      <w:tc>
        <w:tcPr>
          <w:shd w:fill="FFF3BF"/>
          <w:tcBorders><w:top w:val="nil"/><w:left w:val="none"/><w:bottom w:val="nil"/><w:right w:val="none"/></w:tcBorders>
        </w:tcPr>
        <w:p><w:pPr><w:jc w:val="right"/></w:pPr><w:r><w:t>Styled</w:t></w:r></w:p>
      </w:tc>
    </w:tr>
  </w:tbl>
</w:body></w:document>"#,
            None,
            None,
        );

        let parsed = read_docx_bytes(&bytes).expect("styled cell should import");
        let Block::Table(table) = &parsed.sections[0].blocks[0] else {
            panic!("table should import");
        };
        assert_eq!(
            table.rows[0].cells[0].presentation,
            TableCellPresentation {
                background_color: Some("#fff3bf".to_string()),
                text_alignment: Some(ParagraphAlignment::Right),
                border: TableCellBorder::Hidden,
            }
        );
    }

    #[test]
    fn ignores_unsupported_or_mixed_docx_table_cell_presentation() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:tbl>
    <w:tr>
      <w:tc>
        <w:tcPr>
          <w:shd w:fill="FF0000"/>
          <w:tcBorders><w:top w:val="nil"/><w:left w:val="single"/><w:bottom w:val="nil"/><w:right w:val="nil"/></w:tcBorders>
        </w:tcPr>
        <w:p><w:pPr><w:jc w:val="left"/></w:pPr><w:r><w:t>Left</w:t></w:r></w:p>
        <w:p><w:pPr><w:jc w:val="right"/></w:pPr><w:r><w:t>Right</w:t></w:r></w:p>
      </w:tc>
    </w:tr>
  </w:tbl>
</w:body></w:document>"#,
            None,
            None,
        );

        let parsed = read_docx_bytes(&bytes).expect("unsupported styled cell should degrade");
        let Block::Table(table) = &parsed.sections[0].blocks[0] else {
            panic!("table should import");
        };
        let cell = &table.rows[0].cells[0];
        assert_eq!(cell.presentation.background_color, None);
        assert_eq!(cell.presentation.text_alignment, None);
        assert_eq!(cell.presentation.border, TableCellBorder::Visible);
    }

    #[test]
    fn keeps_mixed_default_docx_table_cell_alignment_paragraph_local() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:tbl>
    <w:tr>
      <w:tc>
        <w:p><w:pPr><w:jc w:val="center"/></w:pPr><w:r><w:t>Centered</w:t></w:r></w:p>
        <w:p><w:r><w:t>Default</w:t></w:r></w:p>
      </w:tc>
    </w:tr>
  </w:tbl>
</w:body></w:document>"#,
            None,
            None,
        );

        let parsed = read_docx_bytes(&bytes).expect("mixed default alignment cell should import");
        let Block::Table(table) = &parsed.sections[0].blocks[0] else {
            panic!("table should import");
        };
        let cell = &table.rows[0].cells[0];
        assert_eq!(cell.presentation.text_alignment, None);
        let Block::Paragraph(first) = &cell.blocks[0] else {
            panic!("first paragraph should remain editable");
        };
        let Block::Paragraph(second) = &cell.blocks[1] else {
            panic!("second paragraph should remain editable");
        };
        assert_eq!(first.format.alignment, Some(ParagraphAlignment::Center));
        assert_eq!(second.format.alignment, None);
    }

    #[test]
    fn keeps_docx_table_cell_paragraph_format_when_promoting_common_alignment() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:tbl>
    <w:tr>
      <w:tc>
        <w:p>
          <w:pPr><w:spacing w:line="360" w:lineRule="auto" w:after="283"/><w:ind w:left="454"/><w:jc w:val="center"/></w:pPr>
          <w:r><w:t>Formatted aligned</w:t></w:r>
        </w:p>
      </w:tc>
    </w:tr>
  </w:tbl>
</w:body></w:document>"#,
            None,
            None,
        );

        let parsed = read_docx_bytes(&bytes).expect("formatted aligned cell should import");
        let Block::Table(table) = &parsed.sections[0].blocks[0] else {
            panic!("table should import");
        };
        let cell = &table.rows[0].cells[0];
        assert_eq!(
            cell.presentation.text_alignment,
            Some(ParagraphAlignment::Center)
        );
        let Block::Paragraph(paragraph) = &cell.blocks[0] else {
            panic!("cell paragraph should remain editable");
        };
        assert_eq!(paragraph.format.alignment, None);
        assert_eq!(paragraph.format.line_spacing_per_mille, Some(1500));
        assert_eq!(paragraph.format.spacing_after_mm, Some(5));
        assert_eq!(paragraph.format.indent_start_mm, Some(8));
    }

    #[test]
    fn ignores_docx_table_cell_shading_when_value_is_nil() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:tbl>
    <w:tr>
      <w:tc>
        <w:tcPr><w:shd w:val="nil" w:fill="DBEAFE"/></w:tcPr>
        <w:p><w:r><w:t>Unshaded</w:t></w:r></w:p>
      </w:tc>
    </w:tr>
  </w:tbl>
</w:body></w:document>"#,
            None,
            None,
        );

        let parsed = read_docx_bytes(&bytes).expect("nil shading cell should import");
        let Block::Table(table) = &parsed.sections[0].blocks[0] else {
            panic!("table should import");
        };
        assert_eq!(table.rows[0].cells[0].presentation.background_color, None);
    }

    #[test]
    fn imports_docx_table_grid_widths_as_sanitized_per_mille() {
        let bytes = synthetic_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:tbl>
    <w:tblGrid><w:gridCol w:w="1440"/><w:gridCol w:w="2880"/><w:gridCol w:w="1440"/></w:tblGrid>
    <w:tr>
      <w:tc><w:p><w:r><w:t>A1</w:t></w:r></w:p></w:tc>
      <w:tc><w:p><w:r><w:t>B1</w:t></w:r></w:p></w:tc>
      <w:tc><w:p><w:r><w:t>C1</w:t></w:r></w:p></w:tc>
    </w:tr>
  </w:tbl>
</w:body></w:document>"#,
            None,
            None,
        );

        let document = read_docx_bytes(&bytes).expect("docx should import table widths");

        assert!(document.warnings.is_empty(), "{:?}", document.warnings);
        let Block::Table(table) = &document.sections[0].blocks[0] else {
            panic!("table should import");
        };
        assert_eq!(table.column_widths, vec![250, 500, 250]);
    }

    #[test]
    fn ignores_invalid_or_mismatched_docx_table_grid_widths() {
        let two_cell_row = r#"
    <w:tr>
      <w:tc><w:p><w:r><w:t>A1</w:t></w:r></w:p></w:tc>
      <w:tc><w:p><w:r><w:t>B1</w:t></w:r></w:p></w:tc>
    </w:tr>"#;
        let cases = [
            (
                "mismatched",
                r#"<w:tblGrid><w:gridCol w:w="1000"/><w:gridCol w:w="1000"/><w:gridCol w:w="1000"/></w:tblGrid>"#,
                two_cell_row,
            ),
            (
                "zero",
                r#"<w:tblGrid><w:gridCol w:w="0"/><w:gridCol w:w="1000"/></w:tblGrid>"#,
                two_cell_row,
            ),
            (
                "missing-width",
                r#"<w:tblGrid><w:gridCol/><w:gridCol w:w="1000"/></w:tblGrid>"#,
                two_cell_row,
            ),
            (
                "overflow",
                r#"<w:tblGrid><w:gridCol w:w="184467440737095516160"/><w:gridCol w:w="1000"/></w:tblGrid>"#,
                two_cell_row,
            ),
            (
                "nonnumeric",
                r#"<w:tblGrid><w:gridCol w:w="not-a-number"/><w:gridCol w:w="1000"/></w:tblGrid>"#,
                two_cell_row,
            ),
            (
                "multiple-grid",
                r#"<w:tblGrid><w:gridCol w:w="1000"/><w:gridCol w:w="1000"/></w:tblGrid><w:tblGrid><w:gridCol w:w="2500"/><w:gridCol w:w="7500"/></w:tblGrid>"#,
                two_cell_row,
            ),
            (
                "empty-then-grid",
                r#"<w:tblGrid/><w:tblGrid><w:gridCol w:w="2500"/><w:gridCol w:w="7500"/></w:tblGrid>"#,
                two_cell_row,
            ),
            (
                "non-rectangular",
                r#"<w:tblGrid><w:gridCol w:w="1000"/><w:gridCol w:w="1000"/></w:tblGrid>"#,
                r#"
    <w:tr>
      <w:tc><w:p><w:r><w:t>A1</w:t></w:r></w:p></w:tc>
      <w:tc><w:p><w:r><w:t>B1</w:t></w:r></w:p></w:tc>
    </w:tr>
    <w:tr>
      <w:tc><w:p><w:r><w:t>A2</w:t></w:r></w:p></w:tc>
    </w:tr>"#,
            ),
            (
                "merged-cell",
                r#"<w:tblGrid><w:gridCol w:w="1000"/><w:gridCol w:w="1000"/></w:tblGrid>"#,
                r#"
    <w:tr>
      <w:tc><w:tcPr><w:vMerge w:val="restart"/></w:tcPr><w:p><w:r><w:t>A1</w:t></w:r></w:p></w:tc>
      <w:tc><w:p><w:r><w:t>B1</w:t></w:r></w:p></w:tc>
    </w:tr>
    <w:tr>
      <w:tc><w:tcPr><w:vMerge/></w:tcPr><w:p><w:r><w:t>A2</w:t></w:r></w:p></w:tc>
      <w:tc><w:p><w:r><w:t>B2</w:t></w:r></w:p></w:tc>
    </w:tr>"#,
            ),
            (
                "nested-table",
                r#"<w:tblGrid><w:gridCol w:w="1000"/><w:gridCol w:w="1000"/></w:tblGrid>"#,
                r#"
    <w:tr>
      <w:tc><w:p><w:r><w:t>A1</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Nested</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:tc>
      <w:tc><w:p><w:r><w:t>B1</w:t></w:r></w:p></w:tc>
    </w:tr>"#,
            ),
        ];

        for (case, grid, rows) in cases {
            let document_xml = format!(
                r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body><w:tbl>{grid}{rows}</w:tbl></w:body></w:document>"#
            );
            let bytes = synthetic_docx(&document_xml, None, None);
            let document = read_docx_bytes(&bytes).expect("docx should import table");
            let Block::Table(table) = &document.sections[0].blocks[0] else {
                panic!("table should import for {case}");
            };

            assert!(table.column_widths.is_empty(), "{case}");
            for warning in &document.warnings {
                assert!(!warning.message.contains("not-a-number"), "{case}");
            }
        }
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

    fn assert_all_comment_ids_empty(document: &Document) {
        for section in &document.sections {
            assert_comment_ids_empty_in_blocks(&section.blocks);
        }
    }

    fn assert_comment_ids_empty_in_blocks(blocks: &[Block]) {
        for block in blocks {
            match block {
                Block::Paragraph(paragraph) => {
                    assert!(paragraph
                        .inlines
                        .iter()
                        .all(|inline| inline.comment_ids.is_empty()));
                }
                Block::Heading(heading) => {
                    assert!(heading
                        .inlines
                        .iter()
                        .all(|inline| inline.comment_ids.is_empty()));
                }
                Block::List(list) => {
                    for item in &list.items {
                        assert_comment_ids_empty_in_blocks(&item.blocks);
                    }
                }
                Block::Table(table) => {
                    for row in &table.rows {
                        for cell in &row.cells {
                            assert_comment_ids_empty_in_blocks(&cell.blocks);
                        }
                    }
                }
                Block::TableOfContents(_) | Block::Image(_) | Block::PageBreak => {}
            }
        }
    }

    fn assert_docx_comment_warnings_are_generic(document: &Document, private_body: &str) {
        for warning in &document.warnings {
            assert!(!warning.message.contains("private"));
            assert!(!warning.message.contains("Private"));
            assert!(!warning.message.contains("comments.xml"));
            assert!(!warning.message.contains("word/comments"));
            assert!(!warning.message.contains("C:/"));
            assert!(!warning.message.contains("placeholder"));
            assert!(!warning.message.contains("example.invalid"));
            assert!(!warning.message.contains(private_body));
        }
    }

    fn assert_docx_revision_warnings_are_generic(document: &Document) {
        for warning in &document.warnings {
            assert!(!warning.message.contains("private"));
            assert!(!warning.message.contains("Private"));
            assert!(!warning.message.contains("C:/"));
            assert!(!warning.message.contains("placeholder"));
            assert!(!warning.message.contains("example.invalid"));
            assert!(!warning.message.contains("raw-id"));
            assert!(!warning.message.contains("moveFrom"));
            assert!(!warning.message.contains("delText"));
        }
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

    fn assert_docx_note_warnings_are_generic(document: &Document, private_body: &str) {
        for warning in &document.warnings {
            assert!(!warning.message.contains("private"));
            assert!(!warning.message.contains("Private"));
            assert!(!warning.message.contains("footnotes.xml"));
            assert!(!warning.message.contains("endnotes.xml"));
            assert!(!warning.message.contains("word/footnotes"));
            assert!(!warning.message.contains("word/endnotes"));
            assert!(!warning.message.contains("C:/"));
            assert!(!warning.message.contains("Users"));
            assert!(!warning.message.contains("placeholder"));
            assert!(!warning.message.contains("example.invalid"));
            assert!(!warning.message.contains(private_body));
        }
    }

    fn table_cell_with_text(text: &str) -> TableCell {
        TableCell {
            presentation: Default::default(),
            blocks: vec![Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: StyleId::from("body"),
                format: ParagraphFormat::default(),
                inlines: vec![Inline::text(text)],
            })],
        }
    }

    fn read_zip_text_part(bytes: &[u8], name: &str) -> String {
        let mut archive = ZipArchive::new(Cursor::new(bytes)).expect("written docx should open");
        let mut xml = String::new();
        archive
            .by_name(name)
            .expect("text part should exist")
            .read_to_string(&mut xml)
            .expect("text part should read");
        xml
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
                    &DocxCommentExports::default(),
                    &DocxNoteExports::default(),
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
