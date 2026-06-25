use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub meta: DocumentMeta,
    #[serde(default, skip_serializing_if = "TrackChangesState::is_default")]
    pub track_changes: TrackChangesState,
    pub sections: Vec<Section>,
    pub styles: BTreeMap<StyleId, Style>,
    pub lists: BTreeMap<String, ListDefinition>,
    pub assets: BTreeMap<String, AssetRef>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub comments: BTreeMap<String, CommentThread>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub notes: BTreeMap<String, Note>,
    pub warnings: Vec<DocumentWarning>,
}

impl Document {
    pub fn new_untitled() -> Self {
        Self {
            id: Uuid::new_v4(),
            meta: DocumentMeta::new("Untitled Document"),
            track_changes: TrackChangesState::default(),
            sections: vec![Section::default()],
            styles: default_styles(),
            lists: default_list_definitions(),
            assets: BTreeMap::new(),
            comments: BTreeMap::new(),
            notes: BTreeMap::new(),
            warnings: Vec::new(),
        }
    }

    pub fn stats(&self) -> DocumentStats {
        let mut text = String::new();
        for section in &self.sections {
            for block in &section.blocks {
                block.push_text(&mut text);
                text.push('\n');
            }
        }
        for reference in collect_ordered_note_references(&self.sections) {
            if let Some(note) = self.notes.get(&reference.id) {
                if note.kind != reference.kind {
                    continue;
                }
                text.push('\n');
                text.push_str(&note.body);
            }
        }

        let word_count = text.split_whitespace().count();
        let character_count = text.chars().filter(|ch| !ch.is_control()).count();

        DocumentStats {
            word_count,
            character_count,
            block_count: self
                .sections
                .iter()
                .map(|section| section.blocks.len())
                .sum(),
        }
    }

    pub fn style(&self, style_id: &StyleId) -> Option<&Style> {
        self.styles.get(style_id)
    }

    pub fn register_style(&mut self, style: Style) -> Result<(), DocumentError> {
        validate_non_empty("style id", style.id.as_str())?;
        validate_non_empty("style name", &style.name)?;
        self.styles.insert(style.id.clone(), style);
        self.touch();
        Ok(())
    }

    pub fn apply_command(&mut self, command: DocumentCommand) -> Result<(), DocumentError> {
        match command {
            DocumentCommand::UpdateTitle { title } => {
                validate_non_empty("title", &title)?;
                self.meta.title = title;
                self.touch();
                Ok(())
            }
            DocumentCommand::InsertBlock {
                section_index,
                block_index,
                mut block,
            } => {
                let section = self
                    .sections
                    .get_mut(section_index)
                    .ok_or(DocumentError::SectionOutOfBounds { section_index })?;
                if block_index > section.blocks.len() {
                    return Err(DocumentError::BlockOutOfBounds { block_index });
                }
                normalize_comment_anchors_in_block(&mut block);
                section.blocks.insert(block_index, block);
                self.prune_unanchored_comments();
                self.prune_unreferenced_notes();
                self.touch();
                Ok(())
            }
            DocumentCommand::ReplaceBlock {
                section_index,
                block_index,
                mut block,
            } => {
                let section = self
                    .sections
                    .get_mut(section_index)
                    .ok_or(DocumentError::SectionOutOfBounds { section_index })?;
                let slot = section
                    .blocks
                    .get_mut(block_index)
                    .ok_or(DocumentError::BlockOutOfBounds { block_index })?;
                normalize_comment_anchors_in_block(&mut block);
                *slot = block;
                self.prune_unanchored_comments();
                self.prune_unreferenced_notes();
                self.touch();
                Ok(())
            }
            DocumentCommand::DeleteBlock {
                section_index,
                block_index,
            } => {
                let section = self
                    .sections
                    .get_mut(section_index)
                    .ok_or(DocumentError::SectionOutOfBounds { section_index })?;
                if block_index >= section.blocks.len() {
                    return Err(DocumentError::BlockOutOfBounds { block_index });
                }
                section.blocks.remove(block_index);
                self.prune_unanchored_comments();
                self.prune_unreferenced_notes();
                self.touch();
                Ok(())
            }
            DocumentCommand::InsertOrUpdateTableOfContents {
                section_index,
                block_index,
            } => {
                let used_bookmark_ids = collect_safe_bookmark_id_counts(&self.sections);
                let section = self
                    .sections
                    .get_mut(section_index)
                    .ok_or(DocumentError::SectionOutOfBounds { section_index })?;
                if block_index > section.blocks.len() {
                    return Err(DocumentError::BlockOutOfBounds { block_index });
                }
                let existing_index = section
                    .blocks
                    .iter()
                    .position(|block| matches!(block, Block::TableOfContents(_)));
                let existing_title =
                    existing_index.and_then(|index| match &section.blocks[index] {
                        Block::TableOfContents(table_of_contents) => {
                            sanitized_table_of_contents_title(&table_of_contents.title)
                        }
                        _ => None,
                    });
                let mut toc = build_table_of_contents(section, &used_bookmark_ids);
                if let Some(title) = existing_title {
                    toc.title = title;
                }
                if let Some(existing_index) = existing_index {
                    section.blocks[existing_index] = Block::TableOfContents(toc);
                } else {
                    section
                        .blocks
                        .insert(block_index, Block::TableOfContents(toc));
                }
                self.touch();
                Ok(())
            }
            DocumentCommand::UpdatePageSetup {
                section_index,
                page,
            } => {
                page.validate()?;
                let section = self
                    .sections
                    .get_mut(section_index)
                    .ok_or(DocumentError::SectionOutOfBounds { section_index })?;
                section.page = page;
                self.touch();
                Ok(())
            }
            DocumentCommand::UpdatePageRegion {
                section_index,
                region,
                blocks,
            } => {
                let section = self
                    .sections
                    .get_mut(section_index)
                    .ok_or(DocumentError::SectionOutOfBounds { section_index })?;
                let slot = section.page_regions.region_mut(region);
                if slot.read_only {
                    return Err(DocumentError::ReadOnlyPageRegion { region });
                }
                slot.blocks = blocks;
                self.touch();
                Ok(())
            }
            DocumentCommand::SetDifferentFirstPage {
                section_index,
                enabled,
            } => {
                let section = self
                    .sections
                    .get_mut(section_index)
                    .ok_or(DocumentError::SectionOutOfBounds { section_index })?;
                section.page_regions.different_first_page = enabled;
                self.touch();
                Ok(())
            }
            DocumentCommand::UpdateStyle { style } => self.register_style(style),
            DocumentCommand::SetTrackChangesRecording { enabled } => {
                self.track_changes.recording = enabled;
                self.touch();
                Ok(())
            }
            DocumentCommand::AcceptTrackedChange { id } => {
                self.resolve_tracked_change(&id, TrackedChangeResolution::Accept)
            }
            DocumentCommand::RejectTrackedChange { id } => {
                self.resolve_tracked_change(&id, TrackedChangeResolution::Reject)
            }
            DocumentCommand::AcceptAllTrackedChanges => {
                if self.resolve_all_tracked_changes(TrackedChangeResolution::Accept) {
                    self.touch();
                }
                Ok(())
            }
            DocumentCommand::RejectAllTrackedChanges => {
                if self.resolve_all_tracked_changes(TrackedChangeResolution::Reject) {
                    self.touch();
                }
                Ok(())
            }
            DocumentCommand::InsertNote {
                section_index,
                block_index,
                inline_index,
                id,
                kind,
                body,
                label,
            } => {
                let id = validate_note_id(&id)?;
                if self.notes.contains_key(&id) {
                    return Err(DocumentError::NoteAlreadyExists { id });
                }
                if self.notes.len() >= MAX_NOTES {
                    return Err(DocumentError::TooManyNotes { max: MAX_NOTES });
                }
                let body = validate_note_body(&body)?;
                let label = match label {
                    Some(label) => validate_note_label(&label)?,
                    None => self.next_note_label(kind),
                };
                self.insert_note_reference(
                    section_index,
                    block_index,
                    inline_index,
                    InlineNoteReference {
                        id: id.clone(),
                        kind,
                        label,
                    },
                )?;
                self.notes.insert(id.clone(), Note { id, kind, body });
                self.touch();
                Ok(())
            }
            DocumentCommand::AddNote { id, kind, body } => {
                let id = validate_note_id(&id)?;
                if self.notes.contains_key(&id) {
                    return Err(DocumentError::NoteAlreadyExists { id });
                }
                if self.notes.len() >= MAX_NOTES {
                    return Err(DocumentError::TooManyNotes { max: MAX_NOTES });
                }
                if !note_is_referenced_with_kind(&self.sections, &id, kind) {
                    return Err(DocumentError::NoteNotReferenced { id });
                }
                let body = validate_note_body(&body)?;
                self.notes.insert(id.clone(), Note { id, kind, body });
                self.touch();
                Ok(())
            }
            DocumentCommand::UpdateNote { id, body } => {
                let id = validate_note_id(&id)?;
                let body = validate_note_body(&body)?;
                let note = self
                    .notes
                    .get_mut(&id)
                    .ok_or_else(|| DocumentError::NoteNotFound { id: id.clone() })?;
                note.body = body;
                self.touch();
                Ok(())
            }
            DocumentCommand::DeleteNote { id } => {
                let id = validate_note_id(&id)?;
                if self.notes.remove(&id).is_none() {
                    return Err(DocumentError::NoteNotFound { id });
                }
                self.remove_note_references(&id);
                self.touch();
                Ok(())
            }
            DocumentCommand::AddComment { id, author, body } => {
                let id = validate_comment_id(&id)?;
                if self.comments.contains_key(&id) {
                    return Err(DocumentError::CommentAlreadyExists { id });
                }
                let body = validate_comment_body(&body)?;
                let author = normalize_comment_author(author.as_deref())?;
                if !collect_comment_anchor_ids(&self.sections).contains(&id) {
                    return Err(DocumentError::CommentNotAnchored { id });
                }
                let now = Utc::now();
                self.comments.insert(
                    id.clone(),
                    CommentThread {
                        id,
                        author,
                        body,
                        created_at: now,
                        updated_at: now,
                        resolved: false,
                    },
                );
                self.touch();
                Ok(())
            }
            DocumentCommand::SetCommentResolved { id, resolved } => {
                let id = validate_comment_id(&id)?;
                let comment = self
                    .comments
                    .get_mut(&id)
                    .ok_or_else(|| DocumentError::CommentNotFound { id: id.clone() })?;
                comment.resolved = resolved;
                comment.updated_at = Utc::now();
                self.touch();
                Ok(())
            }
            DocumentCommand::DeleteComment { id } => {
                let id = validate_comment_id(&id)?;
                if self.comments.remove(&id).is_none() {
                    return Err(DocumentError::CommentNotFound { id });
                }
                self.remove_comment_anchors(&id);
                self.touch();
                Ok(())
            }
        }
    }

    fn resolve_tracked_change(
        &mut self,
        id: &str,
        resolution: TrackedChangeResolution,
    ) -> Result<(), DocumentError> {
        let id = validate_tracked_change_id(id)?;
        let mut changed = false;
        for section in &mut self.sections {
            changed |= resolve_tracked_change_in_blocks(&mut section.blocks, &id, resolution);
        }
        if !changed {
            return Err(DocumentError::TrackedChangeNotFound { id });
        }
        self.prune_unanchored_comments();
        self.prune_unreferenced_notes();
        self.touch();
        Ok(())
    }

    fn resolve_all_tracked_changes(&mut self, resolution: TrackedChangeResolution) -> bool {
        let mut changed = false;
        for section in &mut self.sections {
            changed |= resolve_all_tracked_changes_in_blocks(&mut section.blocks, resolution);
        }
        if changed {
            self.prune_unanchored_comments();
            self.prune_unreferenced_notes();
        }
        changed
    }

    fn touch(&mut self) {
        self.meta.modified_at = Utc::now();
    }

    fn remove_comment_anchors(&mut self, comment_id: &str) {
        for section in &mut self.sections {
            remove_comment_anchors_from_blocks(&mut section.blocks, comment_id);
        }
    }

    fn remove_note_references(&mut self, note_id: &str) {
        for section in &mut self.sections {
            remove_note_references_from_blocks(&mut section.blocks, note_id);
        }
    }

    fn insert_note_reference(
        &mut self,
        section_index: usize,
        block_index: usize,
        inline_index: usize,
        reference: InlineNoteReference,
    ) -> Result<(), DocumentError> {
        let section = self
            .sections
            .get_mut(section_index)
            .ok_or(DocumentError::SectionOutOfBounds { section_index })?;
        let block = section
            .blocks
            .get_mut(block_index)
            .ok_or(DocumentError::BlockOutOfBounds { block_index })?;
        let inlines = match block {
            Block::Paragraph(paragraph) => &mut paragraph.inlines,
            Block::Heading(heading) => &mut heading.inlines,
            _ => {
                return Err(DocumentError::UnsupportedBlockForNote {
                    block_type: block.kind_name(),
                })
            }
        };
        if inline_index > inlines.len() {
            return Err(DocumentError::InlineOutOfBounds { inline_index });
        }
        inlines.insert(inline_index, Inline::note_reference(reference));
        Ok(())
    }

    fn next_note_label(&self, kind: NoteKind) -> String {
        let references = collect_ordered_note_references(&self.sections)
            .into_iter()
            .filter(|reference| reference.kind == kind)
            .collect::<Vec<_>>();
        let max_numeric_label = references
            .iter()
            .filter_map(|reference| reference.label.parse::<usize>().ok())
            .max()
            .unwrap_or(0);
        (max_numeric_label.max(references.len()) + 1).to_string()
    }

    fn prune_unanchored_comments(&mut self) {
        if self.comments.is_empty() {
            return;
        }
        let anchored = collect_comment_anchor_ids(&self.sections);
        self.comments.retain(|id, _| anchored.contains(id));
    }

    fn prune_unreferenced_notes(&mut self) {
        if self.notes.is_empty() {
            return;
        }
        let sections = &self.sections;
        self.notes
            .retain(|id, note| note_is_referenced_with_kind(sections, id, note.kind));
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentMeta {
    pub title: String,
    pub subject: Option<String>,
    pub keywords: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub generator: String,
}

impl DocumentMeta {
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            title: title.into(),
            subject: None,
            keywords: Vec::new(),
            created_at: now,
            modified_at: now,
            generator: "900Word".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TrackChangesState {
    #[serde(default, skip_serializing_if = "is_false")]
    pub recording: bool,
}

impl TrackChangesState {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

pub const DEFAULT_COMMENT_AUTHOR: &str = "Local User";
pub const DEFAULT_TRACKED_CHANGE_AUTHOR: &str = "Local User";
pub const MAX_COMMENT_ID_LEN: usize = 64;
pub const MAX_COMMENT_BODY_CHARS: usize = 2_000;
pub const MAX_COMMENT_AUTHOR_CHARS: usize = 80;
pub const MAX_NOTE_ID_LEN: usize = 64;
pub const MAX_NOTE_BODY_CHARS: usize = 4_000;
pub const MAX_NOTE_LABEL_CHARS: usize = 16;
pub const MAX_NOTES: usize = 512;
pub const MAX_TRACKED_CHANGE_ID_LEN: usize = 64;
pub const MAX_TABLE_OF_CONTENTS_TITLE_CHARS: usize = 120;
pub const MAX_TABLE_OF_CONTENTS_ENTRY_TEXT_CHARS: usize = 240;
pub const MAX_TABLE_OF_CONTENTS_ENTRIES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentThread {
    pub id: String,
    pub author: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub resolved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub kind: NoteKind,
    pub body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteKind {
    Footnote,
    Endnote,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineNoteReference {
    pub id: String,
    pub kind: NoteKind,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedChange {
    pub id: String,
    pub kind: TrackedChangeKind,
    pub author: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackedChangeKind {
    Insertion,
    Deletion,
}

impl TrackedChange {
    pub fn new(kind: TrackedChangeKind) -> Self {
        Self {
            id: format!("chg-{}", Uuid::new_v4().simple()),
            kind,
            author: DEFAULT_TRACKED_CHANGE_AUTHOR.to_string(),
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    pub id: Uuid,
    pub blocks: Vec<Block>,
    pub page: PageSetup,
    #[serde(default, skip_serializing_if = "PageRegions::is_default")]
    pub page_regions: PageRegions,
}

impl Default for Section {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            blocks: vec![Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: StyleId::from("body"),
                format: ParagraphFormat::default(),
                inlines: Vec::new(),
            })],
            page: PageSetup::default(),
            page_regions: PageRegions::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSetup {
    pub width_mm: u16,
    pub height_mm: u16,
    pub margin_top_mm: u16,
    pub margin_right_mm: u16,
    pub margin_bottom_mm: u16,
    pub margin_left_mm: u16,
}

impl Default for PageSetup {
    fn default() -> Self {
        Self {
            width_mm: 210,
            height_mm: 297,
            margin_top_mm: 25,
            margin_right_mm: 25,
            margin_bottom_mm: 25,
            margin_left_mm: 25,
        }
    }
}

impl PageSetup {
    pub fn validate(&self) -> Result<(), DocumentError> {
        if !(50..=500).contains(&self.width_mm) || !(50..=500).contains(&self.height_mm) {
            return Err(DocumentError::InvalidPageSetup {
                reason: "page dimensions must be between 50mm and 500mm",
            });
        }
        if [
            self.margin_top_mm,
            self.margin_right_mm,
            self.margin_bottom_mm,
            self.margin_left_mm,
        ]
        .iter()
        .any(|margin| *margin > 100)
        {
            return Err(DocumentError::InvalidPageSetup {
                reason: "page margins must be 100mm or less",
            });
        }
        if self.margin_left_mm + self.margin_right_mm >= self.width_mm {
            return Err(DocumentError::InvalidPageSetup {
                reason: "horizontal margins must fit within page width",
            });
        }
        if self.margin_top_mm + self.margin_bottom_mm >= self.height_mm {
            return Err(DocumentError::InvalidPageSetup {
                reason: "vertical margins must fit within page height",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PageRegions {
    #[serde(default, skip_serializing_if = "PageRegion::is_default")]
    pub header: PageRegion,
    #[serde(default, skip_serializing_if = "PageRegion::is_default")]
    pub footer: PageRegion,
    #[serde(default, skip_serializing_if = "PageRegion::is_default")]
    pub first_header: PageRegion,
    #[serde(default, skip_serializing_if = "PageRegion::is_default")]
    pub first_footer: PageRegion,
    #[serde(default, skip_serializing_if = "is_false")]
    pub different_first_page: bool,
}

impl PageRegions {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }

    pub fn region(&self, kind: PageRegionKind) -> &PageRegion {
        match kind {
            PageRegionKind::Header => &self.header,
            PageRegionKind::Footer => &self.footer,
            PageRegionKind::FirstHeader => &self.first_header,
            PageRegionKind::FirstFooter => &self.first_footer,
        }
    }

    pub fn region_mut(&mut self, kind: PageRegionKind) -> &mut PageRegion {
        match kind {
            PageRegionKind::Header => &mut self.header,
            PageRegionKind::Footer => &mut self.footer,
            PageRegionKind::FirstHeader => &mut self.first_header,
            PageRegionKind::FirstFooter => &mut self.first_footer,
        }
    }

    pub fn has_content(&self) -> bool {
        [
            self.header(),
            self.footer(),
            self.first_header(),
            self.first_footer(),
        ]
        .iter()
        .any(|region| region.has_content())
    }

    pub fn has_read_only_content(&self) -> bool {
        [
            self.header(),
            self.footer(),
            self.first_header(),
            self.first_footer(),
        ]
        .iter()
        .any(|region| region.read_only)
    }

    pub fn header(&self) -> &PageRegion {
        &self.header
    }

    pub fn footer(&self) -> &PageRegion {
        &self.footer
    }

    pub fn first_header(&self) -> &PageRegion {
        &self.first_header
    }

    pub fn first_footer(&self) -> &PageRegion {
        &self.first_footer
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PageRegion {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<PageRegionBlock>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub read_only: bool,
}

impl PageRegion {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }

    pub fn has_content(&self) -> bool {
        !self.blocks.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageRegionKind {
    Header,
    Footer,
    FirstHeader,
    FirstFooter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum PageRegionBlock {
    Paragraph(PageRegionParagraph),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageRegionParagraph {
    pub inlines: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Block {
    Paragraph(Paragraph),
    Heading(Heading),
    TableOfContents(TableOfContents),
    List(ListBlock),
    Table(Table),
    Image(ImageBlock),
    PageBreak,
}

impl Block {
    fn kind_name(&self) -> &'static str {
        match self {
            Block::Paragraph(_) => "Paragraph",
            Block::Heading(_) => "Heading",
            Block::TableOfContents(_) => "TableOfContents",
            Block::List(_) => "List",
            Block::Table(_) => "Table",
            Block::Image(_) => "Image",
            Block::PageBreak => "PageBreak",
        }
    }

    fn push_text(&self, output: &mut String) {
        match self {
            Block::Paragraph(paragraph) => paragraph.push_text(output),
            Block::Heading(heading) => heading.push_text(output),
            Block::TableOfContents(table_of_contents) => table_of_contents.push_text(output),
            Block::List(list) => {
                for item in &list.items {
                    for block in &item.blocks {
                        block.push_text(output);
                        output.push('\n');
                    }
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        for block in &cell.blocks {
                            block.push_text(output);
                            output.push('\t');
                        }
                    }
                    output.push('\n');
                }
            }
            Block::Image(image) => {
                if let Some(alt_text) = &image.alt_text {
                    output.push_str(alt_text);
                }
                if let Some(caption) = image.presentation.caption.as_deref() {
                    if !caption.trim().is_empty() {
                        if image
                            .alt_text
                            .as_deref()
                            .is_some_and(|alt| !alt.trim().is_empty())
                        {
                            output.push('\n');
                        }
                        output.push_str(caption);
                    }
                }
            }
            Block::PageBreak => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paragraph {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmark_id: Option<String>,
    pub style: StyleId,
    #[serde(default, skip_serializing_if = "ParagraphFormat::is_default")]
    pub format: ParagraphFormat,
    pub inlines: Vec<Inline>,
}

impl Paragraph {
    fn push_text(&self, output: &mut String) {
        for inline in &self.inlines {
            inline.push_text(output);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heading {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmark_id: Option<String>,
    pub level: u8,
    pub inlines: Vec<Inline>,
}

impl Heading {
    fn push_text(&self, output: &mut String) {
        for inline in &self.inlines {
            inline.push_text(output);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableOfContents {
    #[serde(default = "default_table_of_contents_title")]
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<TableOfContentsEntry>,
}

impl TableOfContents {
    pub fn new(entries: Vec<TableOfContentsEntry>) -> Self {
        Self {
            title: default_table_of_contents_title(),
            entries,
        }
    }

    fn push_text(&self, output: &mut String) {
        if !self.title.trim().is_empty() {
            output.push_str(self.title.trim());
        }
        for entry in &self.entries {
            if !output.is_empty() {
                output.push('\n');
            }
            for _ in 1..entry.level.clamp(1, 3) {
                output.push_str("  ");
            }
            output.push_str(&entry.text);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableOfContentsEntry {
    pub level: u8,
    pub text: String,
    pub target_bookmark_id: String,
}

fn default_table_of_contents_title() -> String {
    "Contents".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inline {
    pub text: String,
    pub marks: Vec<InlineMark>,
    pub link: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comment_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "InlineStyle::is_default")]
    pub style: InlineStyle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<PageField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_reference: Option<InlineNoteReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracked_change: Option<TrackedChange>,
}

impl Inline {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            marks: Vec::new(),
            link: None,
            comment_ids: Vec::new(),
            style: InlineStyle::default(),
            field: None,
            note_reference: None,
            tracked_change: None,
        }
    }

    pub fn field(field: PageField) -> Self {
        Self {
            text: field.fallback_text().to_string(),
            marks: Vec::new(),
            link: None,
            comment_ids: Vec::new(),
            style: InlineStyle::default(),
            field: Some(field),
            note_reference: None,
            tracked_change: None,
        }
    }

    pub fn note_reference(reference: InlineNoteReference) -> Self {
        Self {
            text: reference.label.clone(),
            marks: Vec::new(),
            link: None,
            comment_ids: Vec::new(),
            style: InlineStyle::default(),
            field: None,
            note_reference: Some(reference),
            tracked_change: None,
        }
    }

    fn push_text(&self, output: &mut String) {
        if let Some(reference) = &self.note_reference {
            output.push_str(&reference.label);
        } else if let Some(field) = self.field {
            output.push_str(field.fallback_text());
        } else {
            output.push_str(&self.text);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageField {
    PageNumber,
    PageCount,
    Date,
}

impl PageField {
    pub fn fallback_text(self) -> &'static str {
        match self {
            PageField::PageNumber => "1",
            PageField::PageCount => "1",
            PageField::Date => "1970-01-01",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct InlineStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size_pt: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight_color: Option<String>,
}

impl InlineStyle {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ParagraphFormat {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alignment: Option<ParagraphAlignment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_spacing_per_mille: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing_before_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing_after_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent_start_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent_end_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_line_indent_mm: Option<i16>,
}

impl ParagraphFormat {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParagraphAlignment {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InlineMark {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Superscript,
    Subscript,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListBlock {
    pub definition_id: String,
    pub items: Vec<ListItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListItem {
    pub level: u8,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListDefinition {
    pub ordered: bool,
    pub marker: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Table {
    pub rows: Vec<TableRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableCell {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageBlock {
    pub asset_id: String,
    #[serde(default, skip_serializing_if = "ImagePresentation::is_default")]
    pub presentation: ImagePresentation,
    pub alt_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImagePresentation {
    #[serde(default)]
    pub alignment: ImageAlignment,
    #[serde(default = "default_image_scale_percent")]
    pub scale_percent: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

impl Default for ImagePresentation {
    fn default() -> Self {
        Self {
            alignment: ImageAlignment::Inline,
            scale_percent: default_image_scale_percent(),
            caption: None,
        }
    }
}

impl ImagePresentation {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageAlignment {
    #[default]
    Inline,
    Left,
    Center,
    Right,
}

fn default_image_scale_percent() -> u16 {
    100
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRef {
    pub id: String,
    pub media_type: String,
    pub byte_len: usize,
    pub bytes: Vec<u8>,
    pub original_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StyleId(String);

impl From<&str> for StyleId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl StyleId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Style {
    pub id: StyleId,
    pub name: String,
    pub kind: StyleKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<StyleId>,
    #[serde(default, skip_serializing_if = "StyleProperties::is_default")]
    pub properties: StyleProperties,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StyleKind {
    Paragraph,
    Character,
    Table,
    Page,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StyleProperties {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paragraph: Option<ParagraphFormat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inline: Option<InlineStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<PageSetup>,
}

impl StyleProperties {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DocumentCommand {
    UpdateTitle {
        title: String,
    },
    InsertBlock {
        section_index: usize,
        block_index: usize,
        block: Block,
    },
    ReplaceBlock {
        section_index: usize,
        block_index: usize,
        block: Block,
    },
    DeleteBlock {
        section_index: usize,
        block_index: usize,
    },
    InsertOrUpdateTableOfContents {
        section_index: usize,
        block_index: usize,
    },
    UpdatePageSetup {
        section_index: usize,
        page: PageSetup,
    },
    UpdatePageRegion {
        section_index: usize,
        region: PageRegionKind,
        blocks: Vec<PageRegionBlock>,
    },
    SetDifferentFirstPage {
        section_index: usize,
        enabled: bool,
    },
    UpdateStyle {
        style: Style,
    },
    SetTrackChangesRecording {
        enabled: bool,
    },
    AcceptTrackedChange {
        id: String,
    },
    RejectTrackedChange {
        id: String,
    },
    AcceptAllTrackedChanges,
    RejectAllTrackedChanges,
    InsertNote {
        section_index: usize,
        block_index: usize,
        inline_index: usize,
        id: String,
        kind: NoteKind,
        body: String,
        label: Option<String>,
    },
    AddNote {
        id: String,
        kind: NoteKind,
        body: String,
    },
    UpdateNote {
        id: String,
        body: String,
    },
    DeleteNote {
        id: String,
    },
    AddComment {
        id: String,
        author: Option<String>,
        body: String,
    },
    SetCommentResolved {
        id: String,
        resolved: bool,
    },
    DeleteComment {
        id: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct UndoStack {
    past: Vec<Document>,
    future: Vec<Document>,
}

impl UndoStack {
    pub fn apply(
        &mut self,
        document: &mut Document,
        command: DocumentCommand,
    ) -> Result<(), DocumentError> {
        self.apply_mutation(document, |document| document.apply_command(command))
    }

    pub fn apply_mutation<F>(
        &mut self,
        document: &mut Document,
        mutation: F,
    ) -> Result<(), DocumentError>
    where
        F: FnOnce(&mut Document) -> Result<(), DocumentError>,
    {
        let before = document.clone();
        mutation(document)?;
        self.past.push(before);
        self.future.clear();
        Ok(())
    }

    pub fn undo(&mut self, document: &mut Document) -> Result<(), DocumentError> {
        let previous = self.past.pop().ok_or(DocumentError::NothingToUndo)?;
        self.future.push(document.clone());
        *document = previous;
        Ok(())
    }

    pub fn redo(&mut self, document: &mut Document) -> Result<(), DocumentError> {
        let next = self.future.pop().ok_or(DocumentError::NothingToRedo)?;
        self.past.push(document.clone());
        *document = next;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentStats {
    pub word_count: usize,
    pub character_count: usize,
    pub block_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DocumentError {
    #[error("{field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("section index {section_index} is out of bounds")]
    SectionOutOfBounds { section_index: usize },
    #[error("block index {block_index} is out of bounds")]
    BlockOutOfBounds { block_index: usize },
    #[error("nothing to undo")]
    NothingToUndo,
    #[error("nothing to redo")]
    NothingToRedo,
    #[error("invalid page setup: {reason}")]
    InvalidPageSetup { reason: &'static str },
    #[error("page region {region:?} is read-only")]
    ReadOnlyPageRegion { region: PageRegionKind },
    #[error("invalid comment id")]
    InvalidCommentId,
    #[error("comment body must not be empty")]
    EmptyCommentBody,
    #[error("comment body is too long; maximum is {max} characters")]
    CommentBodyTooLong { max: usize },
    #[error("comment author is too long; maximum is {max} characters")]
    CommentAuthorTooLong { max: usize },
    #[error("comment {id} already exists")]
    CommentAlreadyExists { id: String },
    #[error("comment {id} was not found")]
    CommentNotFound { id: String },
    #[error("comment {id} has no selected text anchor")]
    CommentNotAnchored { id: String },
    #[error("invalid note id")]
    InvalidNoteId,
    #[error("invalid note label")]
    InvalidNoteLabel,
    #[error("note body must not be empty")]
    EmptyNoteBody,
    #[error("note body is too long; maximum is {max} characters")]
    NoteBodyTooLong { max: usize },
    #[error("note {id} already exists")]
    NoteAlreadyExists { id: String },
    #[error("note {id} was not found")]
    NoteNotFound { id: String },
    #[error("note {id} has no text reference")]
    NoteNotReferenced { id: String },
    #[error("document has too many notes; maximum is {max}")]
    TooManyNotes { max: usize },
    #[error("inline index {inline_index} is out of bounds")]
    InlineOutOfBounds { inline_index: usize },
    #[error("{block_type} blocks do not support note insertion")]
    UnsupportedBlockForNote { block_type: &'static str },
    #[error("invalid tracked change id")]
    InvalidTrackedChangeId,
    #[error("tracked change {id} was not found")]
    TrackedChangeNotFound { id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrackedChangeResolution {
    Accept,
    Reject,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<(), DocumentError> {
    if value.trim().is_empty() {
        Err(DocumentError::EmptyField { field })
    } else {
        Ok(())
    }
}

pub fn sanitize_bookmark_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let mut chars = trimmed.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() || trimmed.len() > 64 {
        return None;
    }
    if chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_') {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn build_table_of_contents(
    section: &mut Section,
    safe_bookmark_counts: &BTreeMap<String, usize>,
) -> TableOfContents {
    let mut reserved_ids = safe_bookmark_counts
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut assigned_ids = BTreeSet::new();
    let mut entries = Vec::new();

    for block in &mut section.blocks {
        let Block::Heading(heading) = block else {
            continue;
        };
        let level = heading.level;
        if !(1..=3).contains(&level) {
            continue;
        }
        let text = heading_plain_text(heading);
        if text.is_empty() {
            continue;
        }

        let current = heading
            .bookmark_id
            .as_deref()
            .and_then(sanitize_bookmark_id);
        let target_bookmark_id = match current {
            Some(id)
                if safe_bookmark_counts.get(&id).copied() == Some(1)
                    && assigned_ids.insert(id.clone()) =>
            {
                if heading.bookmark_id.as_deref() != Some(id.as_str()) {
                    heading.bookmark_id = Some(id.clone());
                }
                id
            }
            _ => {
                let id = unique_toc_bookmark_id(&text, &mut reserved_ids);
                heading.bookmark_id = Some(id.clone());
                assigned_ids.insert(id.clone());
                id
            }
        };

        entries.push(TableOfContentsEntry {
            level,
            text,
            target_bookmark_id,
        });
    }

    TableOfContents::new(entries)
}

fn heading_plain_text(heading: &Heading) -> String {
    let mut text = String::new();
    for inline in &heading.inlines {
        inline.push_text(&mut text);
    }
    text.trim().to_string()
}

fn sanitized_table_of_contents_title(value: &str) -> Option<String> {
    let title = value
        .chars()
        .filter(|ch| !ch.is_control())
        .take(MAX_TABLE_OF_CONTENTS_TITLE_CHARS)
        .collect::<String>();
    let trimmed = title.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn unique_toc_bookmark_id(text: &str, reserved_ids: &mut BTreeSet<String>) -> String {
    let slug = bookmark_slug(text);
    let mut base = format!("toc-{slug}");
    if base.len() > 56 {
        base.truncate(56);
        while base.ends_with('-') {
            base.pop();
        }
    }
    if base == "toc" || sanitize_bookmark_id(&base).is_none() {
        base = "toc-heading".to_string();
    }

    let mut counter = 1_usize;
    loop {
        let candidate = if counter == 1 {
            base.clone()
        } else {
            let suffix = format!("-{counter}");
            let max_base_len = 64_usize.saturating_sub(suffix.len());
            let mut trimmed = base.clone();
            if trimmed.len() > max_base_len {
                trimmed.truncate(max_base_len);
                while trimmed.ends_with('-') {
                    trimmed.pop();
                }
            }
            format!("{trimmed}{suffix}")
        };
        if sanitize_bookmark_id(&candidate).is_some() && reserved_ids.insert(candidate.clone()) {
            return candidate;
        }
        counter += 1;
    }
}

fn bookmark_slug(text: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash && !slug.is_empty() {
            slug.push('-');
            previous_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "heading".to_string()
    } else {
        slug
    }
}

fn collect_safe_bookmark_id_counts(sections: &[Section]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for section in sections {
        collect_safe_bookmark_id_counts_from_blocks(&section.blocks, &mut counts);
    }
    counts
}

fn collect_safe_bookmark_id_counts_from_blocks(
    blocks: &[Block],
    counts: &mut BTreeMap<String, usize>,
) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                if let Some(id) = paragraph
                    .bookmark_id
                    .as_deref()
                    .and_then(sanitize_bookmark_id)
                {
                    *counts.entry(id).or_insert(0) += 1;
                }
            }
            Block::Heading(heading) => {
                if let Some(id) = heading
                    .bookmark_id
                    .as_deref()
                    .and_then(sanitize_bookmark_id)
                {
                    *counts.entry(id).or_insert(0) += 1;
                }
            }
            Block::TableOfContents(_) | Block::Image(_) | Block::PageBreak => {}
            Block::List(list) => {
                for item in &list.items {
                    collect_safe_bookmark_id_counts_from_blocks(&item.blocks, counts);
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_safe_bookmark_id_counts_from_blocks(&cell.blocks, counts);
                    }
                }
            }
        }
    }
}

pub fn validate_comment_id(value: &str) -> Result<String, DocumentError> {
    let trimmed = value.trim();
    let Some(suffix) = trimmed.strip_prefix("cmt-") else {
        return Err(DocumentError::InvalidCommentId);
    };
    if trimmed.len() > MAX_COMMENT_ID_LEN || suffix.is_empty() {
        return Err(DocumentError::InvalidCommentId);
    }
    if suffix
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        Ok(trimmed.to_string())
    } else {
        Err(DocumentError::InvalidCommentId)
    }
}

pub fn validate_tracked_change_id(value: &str) -> Result<String, DocumentError> {
    let trimmed = value.trim();
    let Some(suffix) = trimmed.strip_prefix("chg-") else {
        return Err(DocumentError::InvalidTrackedChangeId);
    };
    if trimmed.len() > MAX_TRACKED_CHANGE_ID_LEN || suffix.is_empty() {
        return Err(DocumentError::InvalidTrackedChangeId);
    }
    if suffix
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        Ok(trimmed.to_string())
    } else {
        Err(DocumentError::InvalidTrackedChangeId)
    }
}

pub fn validate_comment_body(value: &str) -> Result<String, DocumentError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DocumentError::EmptyCommentBody);
    }
    if trimmed.chars().count() > MAX_COMMENT_BODY_CHARS {
        return Err(DocumentError::CommentBodyTooLong {
            max: MAX_COMMENT_BODY_CHARS,
        });
    }
    Ok(trimmed.to_string())
}

pub fn validate_note_id(value: &str) -> Result<String, DocumentError> {
    let trimmed = value.trim();
    let Some(suffix) = trimmed.strip_prefix("note-") else {
        return Err(DocumentError::InvalidNoteId);
    };
    if trimmed.len() > MAX_NOTE_ID_LEN || suffix.is_empty() {
        return Err(DocumentError::InvalidNoteId);
    }
    if suffix
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        Ok(trimmed.to_string())
    } else {
        Err(DocumentError::InvalidNoteId)
    }
}

pub fn validate_note_body(value: &str) -> Result<String, DocumentError> {
    let sanitized = value
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .collect::<String>();
    let trimmed = sanitized.trim();
    if trimmed.is_empty() {
        return Err(DocumentError::EmptyNoteBody);
    }
    if trimmed.chars().count() > MAX_NOTE_BODY_CHARS {
        return Err(DocumentError::NoteBodyTooLong {
            max: MAX_NOTE_BODY_CHARS,
        });
    }
    Ok(trimmed.to_string())
}

pub fn validate_note_label(value: &str) -> Result<String, DocumentError> {
    let sanitized = value
        .chars()
        .filter(|ch| !ch.is_control())
        .collect::<String>();
    let trimmed = sanitized.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_NOTE_LABEL_CHARS {
        return Err(DocumentError::InvalidNoteLabel);
    }
    Ok(trimmed.to_string())
}

pub fn validate_note_reference(
    reference: &InlineNoteReference,
) -> Result<InlineNoteReference, DocumentError> {
    Ok(InlineNoteReference {
        id: validate_note_id(&reference.id)?,
        kind: reference.kind,
        label: validate_note_label(&reference.label)?,
    })
}

pub fn normalize_comment_author(value: Option<&str>) -> Result<String, DocumentError> {
    let sanitized = value
        .unwrap_or(DEFAULT_COMMENT_AUTHOR)
        .chars()
        .filter(|ch| !ch.is_control())
        .collect::<String>();
    let trimmed = sanitized.trim();
    let author = if trimmed.is_empty() {
        DEFAULT_COMMENT_AUTHOR
    } else {
        trimmed
    };
    if author.chars().count() > MAX_COMMENT_AUTHOR_CHARS {
        return Err(DocumentError::CommentAuthorTooLong {
            max: MAX_COMMENT_AUTHOR_CHARS,
        });
    }
    Ok(author.to_string())
}

fn normalize_comment_anchors_in_block(block: &mut Block) {
    match block {
        Block::Paragraph(paragraph) => normalize_inline_metadata(&mut paragraph.inlines),
        Block::Heading(heading) => normalize_inline_metadata(&mut heading.inlines),
        Block::TableOfContents(_) => {}
        Block::List(list) => {
            for item in &mut list.items {
                for block in &mut item.blocks {
                    normalize_comment_anchors_in_block(block);
                }
            }
        }
        Block::Table(table) => {
            for row in &mut table.rows {
                for cell in &mut row.cells {
                    for block in &mut cell.blocks {
                        normalize_comment_anchors_in_block(block);
                    }
                }
            }
        }
        Block::Image(_) | Block::PageBreak => {}
    }
}

fn normalize_inline_metadata(inlines: &mut [Inline]) {
    for inline in inlines {
        let mut seen = BTreeSet::new();
        inline
            .comment_ids
            .retain(|id| validate_comment_id(id).is_ok() && seen.insert(id.clone()));
        inline.note_reference = inline
            .note_reference
            .as_ref()
            .and_then(|reference| validate_note_reference(reference).ok());
        if let Some(reference) = inline.note_reference.as_ref() {
            inline.text = reference.label.clone();
        }
        if let Some(change) = inline.tracked_change.as_mut() {
            if validate_tracked_change_id(&change.id).is_err() {
                inline.tracked_change = None;
                continue;
            }
            change.author = normalize_comment_author(Some(&change.author))
                .unwrap_or_else(|_| DEFAULT_TRACKED_CHANGE_AUTHOR.to_string());
        }
    }
}

fn resolve_tracked_change_in_blocks(
    blocks: &mut [Block],
    change_id: &str,
    resolution: TrackedChangeResolution,
) -> bool {
    let mut changed = false;
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                changed |= resolve_tracked_change_in_inlines(
                    &mut paragraph.inlines,
                    change_id,
                    resolution,
                );
            }
            Block::Heading(heading) => {
                changed |=
                    resolve_tracked_change_in_inlines(&mut heading.inlines, change_id, resolution);
            }
            Block::TableOfContents(_) => {}
            Block::List(list) => {
                for item in &mut list.items {
                    changed |=
                        resolve_tracked_change_in_blocks(&mut item.blocks, change_id, resolution);
                }
            }
            Block::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        changed |= resolve_tracked_change_in_blocks(
                            &mut cell.blocks,
                            change_id,
                            resolution,
                        );
                    }
                }
            }
            Block::Image(_) | Block::PageBreak => {}
        }
    }
    changed
}

fn resolve_all_tracked_changes_in_blocks(
    blocks: &mut [Block],
    resolution: TrackedChangeResolution,
) -> bool {
    let mut changed = false;
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                changed |=
                    resolve_all_tracked_changes_in_inlines(&mut paragraph.inlines, resolution);
            }
            Block::Heading(heading) => {
                changed |= resolve_all_tracked_changes_in_inlines(&mut heading.inlines, resolution);
            }
            Block::TableOfContents(_) => {}
            Block::List(list) => {
                for item in &mut list.items {
                    changed |= resolve_all_tracked_changes_in_blocks(&mut item.blocks, resolution);
                }
            }
            Block::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        changed |=
                            resolve_all_tracked_changes_in_blocks(&mut cell.blocks, resolution);
                    }
                }
            }
            Block::Image(_) | Block::PageBreak => {}
        }
    }
    changed
}

fn resolve_tracked_change_in_inlines(
    inlines: &mut Vec<Inline>,
    change_id: &str,
    resolution: TrackedChangeResolution,
) -> bool {
    resolve_tracked_changes_in_inlines(inlines, resolution, |change| change.id == change_id)
}

fn resolve_all_tracked_changes_in_inlines(
    inlines: &mut Vec<Inline>,
    resolution: TrackedChangeResolution,
) -> bool {
    resolve_tracked_changes_in_inlines(inlines, resolution, |_| true)
}

fn resolve_tracked_changes_in_inlines<F>(
    inlines: &mut Vec<Inline>,
    resolution: TrackedChangeResolution,
    mut matches_change: F,
) -> bool
where
    F: FnMut(&TrackedChange) -> bool,
{
    let mut changed = false;
    let mut next = Vec::with_capacity(inlines.len());
    for mut inline in std::mem::take(inlines) {
        let Some(change) = inline.tracked_change.clone() else {
            next.push(inline);
            continue;
        };
        if !matches_change(&change) {
            next.push(inline);
            continue;
        }

        changed = true;
        let remove_text = matches!(
            (resolution, change.kind),
            (TrackedChangeResolution::Accept, TrackedChangeKind::Deletion)
                | (
                    TrackedChangeResolution::Reject,
                    TrackedChangeKind::Insertion
                )
        );
        if !remove_text {
            inline.tracked_change = None;
            next.push(inline);
        }
    }
    *inlines = next;
    changed
}

fn remove_comment_anchors_from_blocks(blocks: &mut [Block], comment_id: &str) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                remove_comment_anchors_from_inlines(&mut paragraph.inlines, comment_id)
            }
            Block::Heading(heading) => {
                remove_comment_anchors_from_inlines(&mut heading.inlines, comment_id)
            }
            Block::TableOfContents(_) => {}
            Block::List(list) => {
                for item in &mut list.items {
                    remove_comment_anchors_from_blocks(&mut item.blocks, comment_id);
                }
            }
            Block::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        remove_comment_anchors_from_blocks(&mut cell.blocks, comment_id);
                    }
                }
            }
            Block::Image(_) | Block::PageBreak => {}
        }
    }
}

fn remove_comment_anchors_from_inlines(inlines: &mut [Inline], comment_id: &str) {
    for inline in inlines {
        inline.comment_ids.retain(|id| id != comment_id);
    }
}

fn remove_note_references_from_blocks(blocks: &mut [Block], note_id: &str) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                remove_note_references_from_inlines(&mut paragraph.inlines, note_id)
            }
            Block::Heading(heading) => {
                remove_note_references_from_inlines(&mut heading.inlines, note_id)
            }
            Block::TableOfContents(_) => {}
            Block::List(list) => {
                for item in &mut list.items {
                    remove_note_references_from_blocks(&mut item.blocks, note_id);
                }
            }
            Block::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        remove_note_references_from_blocks(&mut cell.blocks, note_id);
                    }
                }
            }
            Block::Image(_) | Block::PageBreak => {}
        }
    }
}

fn remove_note_references_from_inlines(inlines: &mut Vec<Inline>, note_id: &str) {
    inlines.retain(|inline| {
        inline
            .note_reference
            .as_ref()
            .is_none_or(|reference| reference.id != note_id)
    });
}

fn collect_comment_anchor_ids(sections: &[Section]) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for section in sections {
        collect_comment_anchor_ids_from_blocks(&section.blocks, &mut ids);
    }
    ids
}

fn collect_comment_anchor_ids_from_blocks(blocks: &[Block], ids: &mut BTreeSet<String>) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                collect_comment_anchor_ids_from_inlines(&paragraph.inlines, ids)
            }
            Block::Heading(heading) => {
                collect_comment_anchor_ids_from_inlines(&heading.inlines, ids)
            }
            Block::TableOfContents(_) => {}
            Block::List(list) => {
                for item in &list.items {
                    collect_comment_anchor_ids_from_blocks(&item.blocks, ids);
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_comment_anchor_ids_from_blocks(&cell.blocks, ids);
                    }
                }
            }
            Block::Image(_) | Block::PageBreak => {}
        }
    }
}

fn collect_comment_anchor_ids_from_inlines(inlines: &[Inline], ids: &mut BTreeSet<String>) {
    for inline in inlines {
        for id in &inline.comment_ids {
            if validate_comment_id(id).is_ok() {
                ids.insert(id.clone());
            }
        }
    }
}

fn note_is_referenced_with_kind(sections: &[Section], id: &str, kind: NoteKind) -> bool {
    collect_ordered_note_references(sections)
        .into_iter()
        .any(|reference| reference.id == id && reference.kind == kind)
}

pub fn collect_ordered_note_references(sections: &[Section]) -> Vec<InlineNoteReference> {
    let mut references = Vec::new();
    let mut seen = BTreeSet::new();
    for section in sections {
        collect_note_references_from_blocks(&section.blocks, &mut references, &mut seen);
    }
    references
}

fn collect_note_references_from_blocks(
    blocks: &[Block],
    references: &mut Vec<InlineNoteReference>,
    seen: &mut BTreeSet<String>,
) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                collect_note_references_from_inlines(&paragraph.inlines, references, seen)
            }
            Block::Heading(heading) => {
                collect_note_references_from_inlines(&heading.inlines, references, seen)
            }
            Block::TableOfContents(_) => {}
            Block::List(list) => {
                for item in &list.items {
                    collect_note_references_from_blocks(&item.blocks, references, seen);
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_note_references_from_blocks(&cell.blocks, references, seen);
                    }
                }
            }
            Block::Image(_) | Block::PageBreak => {}
        }
    }
}

fn collect_note_references_from_inlines(
    inlines: &[Inline],
    references: &mut Vec<InlineNoteReference>,
    seen: &mut BTreeSet<String>,
) {
    for inline in inlines {
        let Some(reference) = inline
            .note_reference
            .as_ref()
            .and_then(|reference| validate_note_reference(reference).ok())
        else {
            continue;
        };
        if seen.insert(reference.id.clone()) {
            references.push(reference);
        }
    }
}

pub fn default_style_registry() -> BTreeMap<StyleId, Style> {
    let mut styles = BTreeMap::new();
    insert_style(&mut styles, "body", "Normal", StyleKind::Paragraph);
    insert_style(&mut styles, "title", "Title", StyleKind::Paragraph);
    insert_style(&mut styles, "subtitle", "Subtitle", StyleKind::Paragraph);
    insert_style(&mut styles, "heading-1", "Heading 1", StyleKind::Paragraph);
    insert_style(&mut styles, "heading-2", "Heading 2", StyleKind::Paragraph);
    insert_style(&mut styles, "heading-3", "Heading 3", StyleKind::Paragraph);
    insert_style(&mut styles, "quote", "Quote", StyleKind::Paragraph);
    insert_style(&mut styles, "code", "Code", StyleKind::Paragraph);
    insert_style(&mut styles, "caption", "Caption", StyleKind::Paragraph);
    insert_style(&mut styles, "emphasis", "Emphasis", StyleKind::Character);
    insert_style(&mut styles, "strong", "Strong", StyleKind::Character);
    insert_style(&mut styles, "link", "Link", StyleKind::Character);
    insert_style(&mut styles, "highlight", "Highlight", StyleKind::Character);
    insert_style(&mut styles, "default-page", "Default Page", StyleKind::Page);
    insert_style(&mut styles, "first-page", "First Page", StyleKind::Page);
    insert_style(&mut styles, "landscape", "Landscape", StyleKind::Page);
    insert_style(&mut styles, "letterhead", "Letterhead", StyleKind::Page);
    styles
}

fn default_styles() -> BTreeMap<StyleId, Style> {
    default_style_registry()
}

fn insert_style(
    styles: &mut BTreeMap<StyleId, Style>,
    id: &'static str,
    name: &'static str,
    kind: StyleKind,
) {
    let id = StyleId::from(id);
    styles.insert(
        id.clone(),
        Style {
            id,
            name: name.to_string(),
            kind,
            parent: None,
            properties: StyleProperties::default(),
        },
    );
}

fn default_list_definitions() -> BTreeMap<String, ListDefinition> {
    BTreeMap::from([
        (
            "900w-unordered".to_string(),
            ListDefinition {
                ordered: false,
                marker: Some("bullet".to_string()),
            },
        ),
        (
            "900w-ordered".to_string(),
            ListDefinition {
                ordered: true,
                marker: Some("decimal".to_string()),
            },
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_document_starts_with_empty_body_paragraph() {
        let document = Document::new_untitled();

        assert_eq!(document.stats().word_count, 0);
        assert_eq!(document.stats().character_count, 0);
        assert_eq!(document.stats().block_count, 1);
    }

    #[test]
    fn new_document_does_not_create_bookmark_ids_by_default() {
        let document = Document::new_untitled();
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("default block should be a paragraph");
        };

        assert_eq!(paragraph.bookmark_id, None);
    }

    #[test]
    fn table_of_contents_command_generates_safe_unique_heading_targets() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::Paragraph(Paragraph {
                bookmark_id: Some("toc-overview".to_string()),
                style: StyleId::from("body"),
                format: Default::default(),
                inlines: vec![Inline::text("Existing bookmark")],
            }),
            Block::Heading(Heading {
                bookmark_id: None,
                level: 1,
                inlines: vec![Inline::text("Overview")],
            }),
            Block::Heading(Heading {
                bookmark_id: Some("../bad".to_string()),
                level: 2,
                inlines: vec![Inline::text("Details")],
            }),
            Block::Heading(Heading {
                bookmark_id: Some("bm-duplicate".to_string()),
                level: 3,
                inlines: vec![Inline::text("First duplicate")],
            }),
            Block::Heading(Heading {
                bookmark_id: Some("bm-duplicate".to_string()),
                level: 3,
                inlines: vec![Inline::text("Second duplicate")],
            }),
            Block::Heading(Heading {
                bookmark_id: None,
                level: 4,
                inlines: vec![Inline::text("Ignored deep heading")],
            }),
        ];

        document
            .apply_command(DocumentCommand::InsertOrUpdateTableOfContents {
                section_index: 0,
                block_index: 1,
            })
            .expect("toc command should apply");

        let Block::TableOfContents(table_of_contents) = &document.sections[0].blocks[1] else {
            panic!("toc should be inserted at requested block index");
        };
        assert_eq!(table_of_contents.title, "Contents");
        assert_eq!(
            table_of_contents
                .entries
                .iter()
                .map(|entry| (entry.level, entry.text.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (1, "Overview"),
                (2, "Details"),
                (3, "First duplicate"),
                (3, "Second duplicate")
            ]
        );

        let targets = table_of_contents
            .entries
            .iter()
            .map(|entry| entry.target_bookmark_id.clone())
            .collect::<Vec<_>>();
        let unique_targets = targets.iter().cloned().collect::<BTreeSet<_>>();
        assert_eq!(unique_targets.len(), targets.len());
        assert!(!targets.contains(&"toc-overview".to_string()));
        assert!(targets
            .iter()
            .all(|target| sanitize_bookmark_id(target).as_deref() == Some(target.as_str())));

        let heading_bookmarks = document.sections[0]
            .blocks
            .iter()
            .filter_map(|block| match block {
                Block::Heading(heading) if (1..=3).contains(&heading.level) => {
                    heading.bookmark_id.clone()
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(heading_bookmarks, targets);

        let Block::Heading(heading) = &mut document.sections[0].blocks[2] else {
            panic!("first heading should remain after toc");
        };
        heading.inlines = vec![Inline::text("Overview revised")];
        document
            .apply_command(DocumentCommand::InsertOrUpdateTableOfContents {
                section_index: 0,
                block_index: 1,
            })
            .expect("toc update should apply");
        assert_eq!(
            document.sections[0]
                .blocks
                .iter()
                .filter(|block| matches!(block, Block::TableOfContents(_)))
                .count(),
            1
        );
        let Block::TableOfContents(table_of_contents) = &document.sections[0].blocks[1] else {
            panic!("toc should remain in place");
        };
        assert_eq!(table_of_contents.entries[0].text, "Overview revised");
    }

    #[test]
    fn table_of_contents_update_preserves_title_and_undo_redo_restores_state() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::TableOfContents(TableOfContents {
                title: "Contenido".to_string(),
                entries: vec![TableOfContentsEntry {
                    level: 1,
                    text: "Intro".to_string(),
                    target_bookmark_id: "bm-intro".to_string(),
                }],
            }),
            Block::Heading(Heading {
                bookmark_id: Some("bm-intro".to_string()),
                level: 1,
                inlines: vec![Inline::text("Intro revised")],
            }),
        ];
        let before_update = document.clone();
        let mut undo = UndoStack::default();

        undo.apply(
            &mut document,
            DocumentCommand::InsertOrUpdateTableOfContents {
                section_index: 0,
                block_index: 0,
            },
        )
        .expect("toc update should apply");

        let Block::TableOfContents(table_of_contents) = &document.sections[0].blocks[0] else {
            panic!("first block should remain a toc");
        };
        assert_eq!(table_of_contents.title, "Contenido");
        assert_eq!(table_of_contents.entries[0].text, "Intro revised");

        undo.undo(&mut document)
            .expect("undo should restore previous toc");
        assert_eq!(document, before_update);

        undo.redo(&mut document)
            .expect("redo should restore updated toc");
        let Block::TableOfContents(table_of_contents) = &document.sections[0].blocks[0] else {
            panic!("first block should remain a toc after redo");
        };
        assert_eq!(table_of_contents.title, "Contenido");
        assert_eq!(table_of_contents.entries[0].text, "Intro revised");
    }

    #[test]
    fn command_updates_title_and_undo_redo_restores_state() {
        let mut document = Document::new_untitled();
        let original_title = document.meta.title.clone();
        let mut undo = UndoStack::default();

        undo.apply(
            &mut document,
            DocumentCommand::UpdateTitle {
                title: "Draft".to_string(),
            },
        )
        .expect("title update should apply");
        assert_eq!(document.meta.title, "Draft");

        undo.undo(&mut document).expect("undo should restore title");
        assert_eq!(document.meta.title, original_title);

        undo.redo(&mut document).expect("redo should reapply title");
        assert_eq!(document.meta.title, "Draft");
    }

    #[test]
    fn command_updates_page_setup() {
        let mut document = Document::new_untitled();
        let page = PageSetup {
            width_mm: 148,
            height_mm: 210,
            margin_top_mm: 20,
            margin_right_mm: 18,
            margin_bottom_mm: 20,
            margin_left_mm: 18,
        };

        document
            .apply_command(DocumentCommand::UpdatePageSetup {
                section_index: 0,
                page: page.clone(),
            })
            .expect("page setup should update");

        assert_eq!(document.sections[0].page, page);
    }

    #[test]
    fn command_updates_page_region_and_first_page_toggle() {
        let mut document = Document::new_untitled();
        let blocks = vec![PageRegionBlock::Paragraph(PageRegionParagraph {
            inlines: vec![
                Inline::text("Draft page "),
                Inline::field(PageField::PageNumber),
            ],
        })];

        document
            .apply_command(DocumentCommand::UpdatePageRegion {
                section_index: 0,
                region: PageRegionKind::Header,
                blocks: blocks.clone(),
            })
            .expect("header should update");
        document
            .apply_command(DocumentCommand::SetDifferentFirstPage {
                section_index: 0,
                enabled: true,
            })
            .expect("first page toggle should update");

        assert_eq!(document.sections[0].page_regions.header.blocks, blocks);
        assert!(document.sections[0].page_regions.different_first_page);
    }

    #[test]
    fn track_changes_recording_toggle_is_document_state() {
        let mut document = Document::new_untitled();

        document
            .apply_command(DocumentCommand::SetTrackChangesRecording { enabled: true })
            .expect("recording should enable");
        assert!(document.track_changes.recording);

        document
            .apply_command(DocumentCommand::SetTrackChangesRecording { enabled: false })
            .expect("recording should disable");
        assert!(!document.track_changes.recording);
    }

    #[test]
    fn tracked_change_accept_and_reject_individual_changes() {
        let created_at = DateTime::parse_from_rfc3339("2026-06-25T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let insertion = TrackedChange {
            id: "chg-insert".to_string(),
            kind: TrackedChangeKind::Insertion,
            author: DEFAULT_TRACKED_CHANGE_AUTHOR.to_string(),
            created_at,
        };
        let deletion = TrackedChange {
            id: "chg-delete".to_string(),
            kind: TrackedChangeKind::Deletion,
            author: DEFAULT_TRACKED_CHANGE_AUTHOR.to_string(),
            created_at,
        };
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: Default::default(),
            inlines: vec![
                Inline::text("Keep "),
                Inline {
                    text: "added".to_string(),
                    marks: Vec::new(),
                    link: None,
                    comment_ids: Vec::new(),
                    style: Default::default(),
                    field: None,
                    note_reference: None,
                    tracked_change: Some(insertion.clone()),
                },
                Inline {
                    text: " removed".to_string(),
                    marks: Vec::new(),
                    link: None,
                    comment_ids: Vec::new(),
                    style: Default::default(),
                    field: None,
                    note_reference: None,
                    tracked_change: Some(deletion.clone()),
                },
            ],
        })];

        document
            .apply_command(DocumentCommand::AcceptTrackedChange {
                id: insertion.id.clone(),
            })
            .expect("insertion should accept");
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("block should remain a paragraph");
        };
        assert_eq!(paragraph.inlines[1].text, "added");
        assert_eq!(paragraph.inlines[1].tracked_change, None);
        assert!(paragraph.inlines.iter().any(|inline| inline
            .tracked_change
            .as_ref()
            .map(|change| &change.id)
            == Some(&deletion.id)));

        document
            .apply_command(DocumentCommand::RejectTrackedChange {
                id: deletion.id.clone(),
            })
            .expect("deletion should reject");
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("block should remain a paragraph");
        };
        assert_eq!(
            paragraph
                .inlines
                .iter()
                .map(|inline| inline.text.as_str())
                .collect::<String>(),
            "Keep added removed"
        );
        assert!(paragraph
            .inlines
            .iter()
            .all(|inline| inline.tracked_change.is_none()));
    }

    #[test]
    fn tracked_change_accept_all_and_reject_all_cleanup_text_and_comments() {
        let created_at = DateTime::parse_from_rfc3339("2026-06-25T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut reject_document = Document::new_untitled();
        reject_document.comments.insert(
            "cmt-insert".to_string(),
            CommentThread {
                id: "cmt-insert".to_string(),
                author: DEFAULT_COMMENT_AUTHOR.to_string(),
                body: "Inserted range".to_string(),
                created_at,
                updated_at: created_at,
                resolved: false,
            },
        );
        reject_document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: Default::default(),
            inlines: vec![
                Inline::text("Before "),
                Inline {
                    text: "inserted".to_string(),
                    marks: Vec::new(),
                    link: None,
                    comment_ids: vec!["cmt-insert".to_string()],
                    style: Default::default(),
                    field: None,
                    note_reference: None,
                    tracked_change: Some(TrackedChange {
                        id: "chg-insert-all".to_string(),
                        kind: TrackedChangeKind::Insertion,
                        author: DEFAULT_TRACKED_CHANGE_AUTHOR.to_string(),
                        created_at,
                    }),
                },
                Inline {
                    text: " deleted".to_string(),
                    marks: Vec::new(),
                    link: None,
                    comment_ids: Vec::new(),
                    style: Default::default(),
                    field: None,
                    note_reference: None,
                    tracked_change: Some(TrackedChange {
                        id: "chg-delete-all".to_string(),
                        kind: TrackedChangeKind::Deletion,
                        author: DEFAULT_TRACKED_CHANGE_AUTHOR.to_string(),
                        created_at,
                    }),
                },
            ],
        })];

        reject_document
            .apply_command(DocumentCommand::RejectAllTrackedChanges)
            .expect("reject all should resolve");
        let Block::Paragraph(paragraph) = &reject_document.sections[0].blocks[0] else {
            panic!("block should remain a paragraph");
        };
        assert_eq!(
            paragraph
                .inlines
                .iter()
                .map(|inline| inline.text.as_str())
                .collect::<String>(),
            "Before  deleted"
        );
        assert!(reject_document.comments.is_empty());

        let mut accept_document = reject_document.clone();
        accept_document.comments.clear();
        accept_document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: Default::default(),
            inlines: vec![
                Inline::text("Before "),
                Inline {
                    text: "inserted".to_string(),
                    marks: Vec::new(),
                    link: None,
                    comment_ids: Vec::new(),
                    style: Default::default(),
                    field: None,
                    note_reference: None,
                    tracked_change: Some(TrackedChange {
                        id: "chg-insert-all".to_string(),
                        kind: TrackedChangeKind::Insertion,
                        author: DEFAULT_TRACKED_CHANGE_AUTHOR.to_string(),
                        created_at,
                    }),
                },
                Inline {
                    text: " deleted".to_string(),
                    marks: Vec::new(),
                    link: None,
                    comment_ids: Vec::new(),
                    style: Default::default(),
                    field: None,
                    note_reference: None,
                    tracked_change: Some(TrackedChange {
                        id: "chg-delete-all".to_string(),
                        kind: TrackedChangeKind::Deletion,
                        author: DEFAULT_TRACKED_CHANGE_AUTHOR.to_string(),
                        created_at,
                    }),
                },
            ],
        })];

        accept_document
            .apply_command(DocumentCommand::AcceptAllTrackedChanges)
            .expect("accept all should resolve");
        let Block::Paragraph(paragraph) = &accept_document.sections[0].blocks[0] else {
            panic!("block should remain a paragraph");
        };
        assert_eq!(
            paragraph
                .inlines
                .iter()
                .map(|inline| inline.text.as_str())
                .collect::<String>(),
            "Before inserted"
        );
        assert!(paragraph
            .inlines
            .iter()
            .all(|inline| inline.tracked_change.is_none()));
    }

    #[test]
    fn read_only_page_region_rejects_command_update() {
        let mut document = Document::new_untitled();
        document.sections[0].page_regions.footer.read_only = true;

        let err = document
            .apply_command(DocumentCommand::UpdatePageRegion {
                section_index: 0,
                region: PageRegionKind::Footer,
                blocks: vec![PageRegionBlock::Paragraph(PageRegionParagraph {
                    inlines: vec![Inline::text("Replacement")],
                })],
            })
            .expect_err("read-only region should reject updates");

        assert_eq!(
            err,
            DocumentError::ReadOnlyPageRegion {
                region: PageRegionKind::Footer
            }
        );
    }

    #[test]
    fn new_document_defaults_to_a4_page_setup() {
        let document = Document::new_untitled();

        assert_eq!(document.sections[0].page.width_mm, 210);
        assert_eq!(document.sections[0].page.height_mm, 297);
    }

    #[test]
    fn page_setup_rejects_invalid_dimensions_and_margins() {
        let mut document = Document::new_untitled();

        let dimension_error = document
            .apply_command(DocumentCommand::UpdatePageSetup {
                section_index: 0,
                page: PageSetup {
                    width_mm: 10,
                    ..PageSetup::default()
                },
            })
            .expect_err("tiny page width should fail");
        assert_eq!(
            dimension_error,
            DocumentError::InvalidPageSetup {
                reason: "page dimensions must be between 50mm and 500mm"
            }
        );

        let margin_error = document
            .apply_command(DocumentCommand::UpdatePageSetup {
                section_index: 0,
                page: PageSetup {
                    width_mm: 100,
                    margin_left_mm: 60,
                    margin_right_mm: 40,
                    ..PageSetup::default()
                },
            })
            .expect_err("margins that consume width should fail");
        assert_eq!(
            margin_error,
            DocumentError::InvalidPageSetup {
                reason: "horizontal margins must fit within page width"
            }
        );
    }

    #[test]
    fn empty_title_is_rejected() {
        let mut document = Document::new_untitled();

        let err = document
            .apply_command(DocumentCommand::UpdateTitle {
                title: " ".to_string(),
            })
            .expect_err("blank title should fail");

        assert_eq!(err, DocumentError::EmptyField { field: "title" });
    }

    #[test]
    fn image_caption_and_alt_contribute_to_document_text_stats() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Image(ImageBlock {
            asset_id: "image-1.png".to_string(),
            presentation: ImagePresentation {
                alignment: ImageAlignment::Center,
                scale_percent: 80,
                caption: Some("Visible caption".to_string()),
            },
            alt_text: Some("Chart alt".to_string()),
        })];

        let stats = document.stats();

        assert_eq!(stats.word_count, 4);
        assert_eq!(stats.character_count, "Chart altVisible caption".len());
    }

    #[test]
    fn default_style_registry_contains_authoring_styles() {
        let document = Document::new_untitled();

        assert_eq!(
            document
                .style(&StyleId::from("body"))
                .map(|style| style.name.as_str()),
            Some("Normal")
        );
        assert_eq!(
            document
                .style(&StyleId::from("heading-1"))
                .map(|style| style.kind),
            Some(StyleKind::Paragraph)
        );
        assert_eq!(
            document
                .style(&StyleId::from("heading-3"))
                .map(|style| style.name.as_str()),
            Some("Heading 3")
        );
        assert_eq!(
            document
                .style(&StyleId::from("strong"))
                .map(|style| style.kind),
            Some(StyleKind::Character)
        );
        assert_eq!(
            document
                .style(&StyleId::from("landscape"))
                .map(|style| style.kind),
            Some(StyleKind::Page)
        );
        assert!(document.lists.contains_key("900w-unordered"));
        assert!(document.lists.contains_key("900w-ordered"));
    }

    #[test]
    fn style_registry_rejects_empty_style_name() {
        let mut document = Document::new_untitled();

        let err = document
            .register_style(Style {
                id: StyleId::from("caption"),
                name: " ".to_string(),
                kind: StyleKind::Paragraph,
                parent: None,
                properties: StyleProperties::default(),
            })
            .expect_err("blank style name should fail");

        assert_eq!(
            err,
            DocumentError::EmptyField {
                field: "style name"
            }
        );
    }

    #[test]
    fn command_updates_style_properties() {
        let mut document = Document::new_untitled();

        document
            .apply_command(DocumentCommand::UpdateStyle {
                style: Style {
                    id: StyleId::from("quote"),
                    name: "Quote".to_string(),
                    kind: StyleKind::Paragraph,
                    parent: None,
                    properties: StyleProperties {
                        paragraph: Some(ParagraphFormat {
                            alignment: Some(ParagraphAlignment::Justify),
                            line_spacing_per_mille: Some(1500),
                            spacing_before_mm: Some(2),
                            spacing_after_mm: Some(4),
                            indent_start_mm: Some(8),
                            indent_end_mm: None,
                            first_line_indent_mm: Some(-3),
                        }),
                        inline: None,
                        page: None,
                    },
                },
            })
            .expect("style update should apply");

        assert_eq!(
            document
                .style(&StyleId::from("quote"))
                .and_then(|style| style.properties.paragraph.as_ref())
                .and_then(|format| format.alignment),
            Some(ParagraphAlignment::Justify)
        );
    }

    #[test]
    fn comment_command_defaults_author_and_validates_body_and_id() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: Default::default(),
            inlines: vec![Inline {
                text: "Reviewed range".to_string(),
                marks: Vec::new(),
                link: None,
                comment_ids: vec!["cmt-1234".to_string()],
                style: Default::default(),
                field: None,
                note_reference: None,
                tracked_change: None,
            }],
        })];

        document
            .apply_command(DocumentCommand::AddComment {
                id: "cmt-1234".to_string(),
                author: None,
                body: "  Please verify this sentence.  ".to_string(),
            })
            .expect("valid comment should be added");

        let comment = document.comments.get("cmt-1234").expect("comment exists");
        assert_eq!(comment.author, DEFAULT_COMMENT_AUTHOR);
        assert_eq!(comment.body, "Please verify this sentence.");
        assert!(!comment.resolved);

        let duplicate = document
            .apply_command(DocumentCommand::AddComment {
                id: "cmt-1234".to_string(),
                author: None,
                body: "Another comment".to_string(),
            })
            .expect_err("duplicate comment id should fail");
        assert_eq!(
            duplicate,
            DocumentError::CommentAlreadyExists {
                id: "cmt-1234".to_string()
            }
        );

        let invalid_id = document
            .apply_command(DocumentCommand::AddComment {
                id: "../bad".to_string(),
                author: None,
                body: "Body".to_string(),
            })
            .expect_err("unsafe comment id should fail");
        assert_eq!(invalid_id, DocumentError::InvalidCommentId);

        let empty_body = document
            .apply_command(DocumentCommand::AddComment {
                id: "cmt-empty".to_string(),
                author: None,
                body: "   ".to_string(),
            })
            .expect_err("empty body should fail");
        assert_eq!(empty_body, DocumentError::EmptyCommentBody);

        let long_body = document
            .apply_command(DocumentCommand::AddComment {
                id: "cmt-long".to_string(),
                author: None,
                body: "x".repeat(MAX_COMMENT_BODY_CHARS + 1),
            })
            .expect_err("long body should fail");
        assert_eq!(
            long_body,
            DocumentError::CommentBodyTooLong {
                max: MAX_COMMENT_BODY_CHARS
            }
        );

        let unanchored = document
            .apply_command(DocumentCommand::AddComment {
                id: "cmt-unanchored".to_string(),
                author: None,
                body: "Body".to_string(),
            })
            .expect_err("comment without selected text anchor should fail");
        assert_eq!(
            unanchored,
            DocumentError::CommentNotAnchored {
                id: "cmt-unanchored".to_string()
            }
        );
    }

    #[test]
    fn footnote_insert_command_updates_text_stats_and_undo_stack() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: Default::default(),
            inlines: vec![Inline::text("Claim")],
        })];
        let mut undo = UndoStack::default();

        undo.apply(
            &mut document,
            DocumentCommand::InsertNote {
                section_index: 0,
                block_index: 0,
                inline_index: 1,
                id: "note-source".to_string(),
                kind: NoteKind::Footnote,
                body: "Source detail".to_string(),
                label: None,
            },
        )
        .expect("footnote should insert");

        let note = document.notes.get("note-source").expect("note exists");
        assert_eq!(note.kind, NoteKind::Footnote);
        assert_eq!(note.body, "Source detail");
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
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
        assert_eq!(document.stats().word_count, 3);

        undo.undo(&mut document)
            .expect("undo should remove footnote");
        assert!(document.notes.is_empty());
        undo.redo(&mut document)
            .expect("redo should restore footnote");
        assert!(document.notes.contains_key("note-source"));

        document
            .apply_command(DocumentCommand::InsertNote {
                section_index: 0,
                block_index: 0,
                inline_index: 2,
                id: "note-second".to_string(),
                kind: NoteKind::Footnote,
                body: "Second detail".to_string(),
                label: None,
            })
            .expect("second footnote should insert");
        document
            .apply_command(DocumentCommand::DeleteNote {
                id: "note-source".to_string(),
            })
            .expect("first footnote should delete");
        document
            .apply_command(DocumentCommand::InsertNote {
                section_index: 0,
                block_index: 0,
                inline_index: 2,
                id: "note-third".to_string(),
                kind: NoteKind::Footnote,
                body: "Third detail".to_string(),
                label: None,
            })
            .expect("third footnote should insert");
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("block should remain a paragraph");
        };
        let labels = paragraph
            .inlines
            .iter()
            .filter_map(|inline| {
                inline
                    .note_reference
                    .as_ref()
                    .map(|reference| reference.label.as_str())
            })
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["2", "3"]);
    }

    #[test]
    fn footnote_add_update_delete_validate_and_prune_references() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: Default::default(),
            inlines: vec![
                Inline::text("Body"),
                Inline::note_reference(InlineNoteReference {
                    id: "note-mismatch".to_string(),
                    kind: NoteKind::Footnote,
                    label: "1".to_string(),
                }),
                Inline::note_reference(InlineNoteReference {
                    id: "note-existing".to_string(),
                    kind: NoteKind::Endnote,
                    label: "i".to_string(),
                }),
            ],
        })];

        document
            .apply_command(DocumentCommand::AddNote {
                id: "note-existing".to_string(),
                kind: NoteKind::Endnote,
                body: "End matter".to_string(),
            })
            .expect("referenced endnote should add");

        let mismatched = document
            .apply_command(DocumentCommand::AddNote {
                id: "note-mismatch".to_string(),
                kind: NoteKind::Endnote,
                body: "Wrong kind".to_string(),
            })
            .expect_err("mismatched note kind should fail");
        assert_eq!(
            mismatched,
            DocumentError::NoteNotReferenced {
                id: "note-mismatch".to_string()
            }
        );

        document
            .apply_command(DocumentCommand::UpdateNote {
                id: "note-existing".to_string(),
                body: "Updated end matter".to_string(),
            })
            .expect("note should update");
        assert_eq!(document.notes["note-existing"].body, "Updated end matter");

        let unreferenced = document
            .apply_command(DocumentCommand::AddNote {
                id: "note-missing".to_string(),
                kind: NoteKind::Footnote,
                body: "Missing reference".to_string(),
            })
            .expect_err("unreferenced note should fail");
        assert_eq!(
            unreferenced,
            DocumentError::NoteNotReferenced {
                id: "note-missing".to_string()
            }
        );

        document
            .apply_command(DocumentCommand::DeleteNote {
                id: "note-existing".to_string(),
            })
            .expect("note should delete");
        assert!(document.notes.is_empty());
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("block should remain a paragraph");
        };
        assert_eq!(paragraph.inlines.len(), 2);
        assert!(paragraph.inlines.iter().all(|inline| inline
            .note_reference
            .as_ref()
            .is_none_or(|reference| reference.id != "note-existing")));
    }

    #[test]
    fn comment_resolve_unresolve_and_delete_clean_anchors() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: Default::default(),
            inlines: vec![Inline {
                text: "Reviewed range".to_string(),
                marks: Vec::new(),
                link: None,
                comment_ids: vec!["cmt-review".to_string()],
                style: Default::default(),
                field: None,
                note_reference: None,
                tracked_change: None,
            }],
        })];

        document
            .apply_command(DocumentCommand::AddComment {
                id: "cmt-review".to_string(),
                author: Some("Local User".to_string()),
                body: "Check wording".to_string(),
            })
            .expect("comment should add");
        document
            .apply_command(DocumentCommand::SetCommentResolved {
                id: "cmt-review".to_string(),
                resolved: true,
            })
            .expect("comment should resolve");
        assert!(document.comments["cmt-review"].resolved);
        document
            .apply_command(DocumentCommand::SetCommentResolved {
                id: "cmt-review".to_string(),
                resolved: false,
            })
            .expect("comment should unresolve");
        assert!(!document.comments["cmt-review"].resolved);

        document
            .apply_command(DocumentCommand::DeleteComment {
                id: "cmt-review".to_string(),
            })
            .expect("comment should delete");

        assert!(!document.comments.contains_key("cmt-review"));
        let Block::Paragraph(paragraph) = &document.sections[0].blocks[0] else {
            panic!("block should remain a paragraph");
        };
        assert!(paragraph.inlines[0].comment_ids.is_empty());
    }

    #[test]
    fn replacing_anchored_content_prunes_unanchored_comment_metadata() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: StyleId::from("body"),
            format: Default::default(),
            inlines: vec![Inline {
                text: "Commented text".to_string(),
                marks: Vec::new(),
                link: None,
                comment_ids: vec!["cmt-delete-me".to_string()],
                style: Default::default(),
                field: None,
                note_reference: None,
                tracked_change: None,
            }],
        })];
        document
            .apply_command(DocumentCommand::AddComment {
                id: "cmt-delete-me".to_string(),
                author: None,
                body: "Remove with anchor".to_string(),
            })
            .expect("comment should add");

        document
            .apply_command(DocumentCommand::ReplaceBlock {
                section_index: 0,
                block_index: 0,
                block: Block::Paragraph(Paragraph {
                    bookmark_id: None,
                    style: StyleId::from("body"),
                    format: Default::default(),
                    inlines: vec![Inline::text("Plain replacement")],
                }),
            })
            .expect("block should replace");

        assert!(document.comments.is_empty());
    }
}
