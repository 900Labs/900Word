use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub meta: DocumentMeta,
    pub sections: Vec<Section>,
    pub styles: BTreeMap<StyleId, Style>,
    pub lists: BTreeMap<String, ListDefinition>,
    pub assets: BTreeMap<String, AssetRef>,
    pub warnings: Vec<DocumentWarning>,
}

impl Document {
    pub fn new_untitled() -> Self {
        Self {
            id: Uuid::new_v4(),
            meta: DocumentMeta::new("Untitled Document"),
            sections: vec![Section::default()],
            styles: default_styles(),
            lists: default_list_definitions(),
            assets: BTreeMap::new(),
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
                block,
            } => {
                let section = self
                    .sections
                    .get_mut(section_index)
                    .ok_or(DocumentError::SectionOutOfBounds { section_index })?;
                if block_index > section.blocks.len() {
                    return Err(DocumentError::BlockOutOfBounds { block_index });
                }
                section.blocks.insert(block_index, block);
                self.touch();
                Ok(())
            }
            DocumentCommand::ReplaceBlock {
                section_index,
                block_index,
                block,
            } => {
                let section = self
                    .sections
                    .get_mut(section_index)
                    .ok_or(DocumentError::SectionOutOfBounds { section_index })?;
                let slot = section
                    .blocks
                    .get_mut(block_index)
                    .ok_or(DocumentError::BlockOutOfBounds { block_index })?;
                *slot = block;
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
            DocumentCommand::UpdateStyle { style } => self.register_style(style),
        }
    }

    fn touch(&mut self) {
        self.meta.modified_at = Utc::now();
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    pub id: Uuid,
    pub blocks: Vec<Block>,
    pub page: PageSetup,
}

impl Default for Section {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            blocks: vec![Block::Paragraph(Paragraph {
                style: StyleId::from("body"),
                format: ParagraphFormat::default(),
                inlines: Vec::new(),
            })],
            page: PageSetup::default(),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Block {
    Paragraph(Paragraph),
    Heading(Heading),
    List(ListBlock),
    Table(Table),
    Image(ImageBlock),
    PageBreak,
}

impl Block {
    fn push_text(&self, output: &mut String) {
        match self {
            Block::Paragraph(paragraph) => paragraph.push_text(output),
            Block::Heading(heading) => heading.push_text(output),
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
            }
            Block::PageBreak => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paragraph {
    pub style: StyleId,
    #[serde(default, skip_serializing_if = "ParagraphFormat::is_default")]
    pub format: ParagraphFormat,
    pub inlines: Vec<Inline>,
}

impl Paragraph {
    fn push_text(&self, output: &mut String) {
        for inline in &self.inlines {
            output.push_str(&inline.text);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heading {
    pub level: u8,
    pub inlines: Vec<Inline>,
}

impl Heading {
    fn push_text(&self, output: &mut String) {
        for inline in &self.inlines {
            output.push_str(&inline.text);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inline {
    pub text: String,
    pub marks: Vec<InlineMark>,
    pub link: Option<String>,
    #[serde(default, skip_serializing_if = "InlineStyle::is_default")]
    pub style: InlineStyle,
}

impl Inline {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            marks: Vec::new(),
            link: None,
            style: InlineStyle::default(),
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
    pub alt_text: Option<String>,
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
    UpdatePageSetup {
        section_index: usize,
        page: PageSetup,
    },
    UpdateStyle {
        style: Style,
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
        let before = document.clone();
        document.apply_command(command)?;
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
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<(), DocumentError> {
    if value.trim().is_empty() {
        Err(DocumentError::EmptyField { field })
    } else {
        Ok(())
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
}
