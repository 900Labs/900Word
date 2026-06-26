use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use uuid::Uuid;
use word_core::{
    AssetRef, Block, CommentThread, Document, Heading, ImageAlignment, ImageBlock,
    ImagePresentation, Inline, InlineMark, InlineNoteReference, ListBlock, ListItem, Note,
    NoteKind, PageField, PageRegionBlock, PageRegionParagraph, Paragraph, ParagraphAlignment,
    ParagraphFormat, StyleId, Table, TableCell, TableCellBorder, TableCellPresentation,
    TableOfContents, TableOfContentsEntry, TableRow, TrackChangesState, TrackedChange,
    TrackedChangeKind,
};

pub const GENERATED_MULTILINGUAL_JSON: &str =
    include_str!("../fixtures/generated-multilingual.json");

pub const SAMPLE_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1, 13, 10,
    45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

pub fn multilingual_sample() -> Document {
    let mut document = Document::new_untitled();
    let timestamp = fixture_timestamp();
    document.id = fixture_id(1);
    document.meta.title = "Generated multilingual sample".to_string();
    document.meta.created_at = timestamp;
    document.meta.modified_at = timestamp;
    document.sections[0].id = fixture_id(2);
    document.sections[0].blocks = vec![
        Block::Heading(Heading {
            bookmark_id: None,
            level: 1,
            inlines: vec![Inline::text("Generated sample")],
        }),
        Block::Paragraph(word_core::Paragraph {
            bookmark_id: None,
            style: word_core::StyleId::from("body"),
            format: Default::default(),
            inlines: vec![Inline::text("Hello offline world.")],
        }),
        Block::Paragraph(word_core::Paragraph {
            bookmark_id: None,
            style: word_core::StyleId::from("body"),
            format: Default::default(),
            inlines: vec![Inline::text("مرحبا بالعالم")],
        }),
        Block::Paragraph(word_core::Paragraph {
            bookmark_id: None,
            style: word_core::StyleId::from("body"),
            format: Default::default(),
            inlines: vec![Inline::text("你好，世界")],
        }),
    ];
    document
}

pub fn compatibility_sample() -> Document {
    let mut document = Document::new_untitled();
    let timestamp = fixture_timestamp();
    document.id = fixture_id(10);
    document.meta.title = "900Word compatibility sample".to_string();
    document.meta.subject = Some("Generated placeholder compatibility fixture".to_string());
    document.meta.keywords = vec![
        "900Word".to_string(),
        "compatibility".to_string(),
        "generated".to_string(),
    ];
    document.meta.created_at = timestamp;
    document.meta.modified_at = timestamp;
    document.track_changes = TrackChangesState { recording: true };
    document.sections[0].id = fixture_id(11);
    document.sections[0].page_regions.header.blocks =
        vec![page_region_paragraph(vec![Inline::text(
            "900Word compatibility sample",
        )])];
    document.sections[0].page_regions.footer.blocks = vec![page_region_paragraph(vec![
        Inline::text("Page "),
        Inline::field(PageField::PageNumber),
        Inline::text(" of "),
        Inline::field(PageField::PageCount),
        Inline::text(" - "),
        Inline::field(PageField::Date),
    ])];

    document.assets.insert(
        "compatibility-image.png".to_string(),
        AssetRef {
            id: "compatibility-image.png".to_string(),
            media_type: "image/png".to_string(),
            byte_len: SAMPLE_PNG.len(),
            bytes: SAMPLE_PNG.to_vec(),
            original_name: None,
        },
    );

    document.comments = BTreeMap::from([(
        "comment-compat-1".to_string(),
        CommentThread {
            id: "comment-compat-1".to_string(),
            author: "Local User".to_string(),
            body: "Generated review note for compatibility testing.".to_string(),
            created_at: timestamp,
            updated_at: timestamp,
            resolved: false,
        },
    )]);
    document.notes = BTreeMap::from([(
        "note-compat-1".to_string(),
        Note {
            id: "note-compat-1".to_string(),
            kind: NoteKind::Footnote,
            body: "Generated footnote body for compatibility testing.".to_string(),
        },
    )]);

    document.sections[0].blocks = vec![
        heading_block_with_bookmark(1, "Generated Compatibility Report", "compat-title"),
        Block::TableOfContents(TableOfContents::new(vec![
            TableOfContentsEntry {
                level: 1,
                text: "Generated Compatibility Report".to_string(),
                target_bookmark_id: "compat-title".to_string(),
            },
            TableOfContentsEntry {
                level: 2,
                text: "Document Content".to_string(),
                target_bookmark_id: "compat-content".to_string(),
            },
            TableOfContentsEntry {
                level: 2,
                text: "Review Features".to_string(),
                target_bookmark_id: "compat-review".to_string(),
            },
        ])),
        paragraph_block_with_inlines(vec![
            Inline::text("This generated placeholder file exercises "),
            marked_inline("900Word", vec![InlineMark::Bold]),
            Inline::text(" export features without real private content."),
        ]),
        heading_block_with_bookmark(2, "Document Content", "compat-content"),
        paragraph_block_with_format(
            "The paragraph uses centered alignment, spacing, and safe direct styling.",
            ParagraphFormat {
                alignment: Some(ParagraphAlignment::Center),
                line_spacing_per_mille: Some(1_150),
                spacing_before_mm: Some(2),
                spacing_after_mm: Some(3),
                ..ParagraphFormat::default()
            },
        ),
        paragraph_block_with_inlines(vec![
            styled_inline("Colored text", Some("#1d4ed8"), None),
            Inline::text(" and "),
            styled_inline("highlighted text", None, Some("#fff3bf")),
            Inline::text(" remain generated fixture content."),
        ]),
        Block::List(ListBlock {
            definition_id: "900w-ordered".to_string(),
            items: vec![
                list_item(1, "Generated first task"),
                list_item(2, "Generated nested task"),
                list_item(1, "Generated final task"),
            ],
        }),
        Block::Table(Table {
            column_widths: vec![250, 250, 500],
            rows: vec![
                table_row(vec![
                    table_cell("Item", Some("#f1f5f9"), Some(ParagraphAlignment::Center)),
                    table_cell("Status", Some("#f1f5f9"), Some(ParagraphAlignment::Center)),
                    table_cell("Notes", Some("#f1f5f9"), Some(ParagraphAlignment::Center)),
                ]),
                table_row(vec![
                    table_cell("ODT", None, None),
                    table_cell("Ready", Some("#dcfce7"), Some(ParagraphAlignment::Center)),
                    table_cell("Native saved format", None, None),
                ]),
                table_row(vec![
                    table_cell("DOCX", None, None),
                    table_cell(
                        "Conversion",
                        Some("#fff3bf"),
                        Some(ParagraphAlignment::Center),
                    ),
                    table_cell("Generated compatibility export", None, None),
                ]),
            ],
        }),
        Block::Image(ImageBlock {
            asset_id: "compatibility-image.png".to_string(),
            presentation: ImagePresentation {
                alignment: ImageAlignment::Center,
                scale_percent: 125,
                caption: Some("Generated placeholder image caption".to_string()),
            },
            alt_text: Some("Generated placeholder image".to_string()),
        }),
        heading_block_with_bookmark(2, "Review Features", "compat-review"),
        paragraph_block_with_inlines(vec![
            comment_inline("This sentence has a generated comment.", "comment-compat-1"),
            Inline::text(" "),
            Inline::note_reference(InlineNoteReference {
                id: "note-compat-1".to_string(),
                kind: NoteKind::Footnote,
                label: "1".to_string(),
            }),
        ]),
        paragraph_block_with_inlines(vec![
            tracked_inline(
                "Generated inserted text.",
                TrackedChangeKind::Insertion,
                "chg-compat-insert-1",
                timestamp,
            ),
            Inline::text(" "),
            tracked_inline(
                "Generated deleted text.",
                TrackedChangeKind::Deletion,
                "chg-compat-delete-1",
                timestamp,
            ),
        ]),
    ];
    document
}

pub fn multilingual_sample_json() -> String {
    serde_json::to_string_pretty(&multilingual_sample()).expect("generated fixture must serialize")
}

pub fn load_generated_multilingual_json() -> Document {
    serde_json::from_str(GENERATED_MULTILINGUAL_JSON).expect("generated fixture JSON must parse")
}

fn fixture_id(suffix: u128) -> Uuid {
    Uuid::from_u128(0x00000000_0000_4000_8000_000000000000 + suffix)
}

fn fixture_timestamp() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
        .expect("fixture timestamp must parse")
        .with_timezone(&Utc)
}

fn page_region_paragraph(inlines: Vec<Inline>) -> PageRegionBlock {
    PageRegionBlock::Paragraph(PageRegionParagraph { inlines })
}

fn heading_block_with_bookmark(level: u8, text: &str, bookmark_id: &str) -> Block {
    Block::Heading(Heading {
        bookmark_id: Some(bookmark_id.to_string()),
        level,
        inlines: vec![Inline::text(text)],
    })
}

fn paragraph_block_with_inlines(inlines: Vec<Inline>) -> Block {
    Block::Paragraph(Paragraph {
        bookmark_id: None,
        style: StyleId::from("body"),
        format: ParagraphFormat::default(),
        inlines,
    })
}

fn paragraph_block_with_format(text: &str, format: ParagraphFormat) -> Block {
    Block::Paragraph(Paragraph {
        bookmark_id: None,
        style: StyleId::from("body"),
        format,
        inlines: vec![Inline::text(text)],
    })
}

fn marked_inline(text: &str, marks: Vec<InlineMark>) -> Inline {
    let mut inline = Inline::text(text);
    inline.marks = marks;
    inline
}

fn styled_inline(text: &str, text_color: Option<&str>, highlight_color: Option<&str>) -> Inline {
    let mut inline = Inline::text(text);
    inline.style.text_color = text_color.map(str::to_string);
    inline.style.highlight_color = highlight_color.map(str::to_string);
    inline
}

fn comment_inline(text: &str, comment_id: &str) -> Inline {
    let mut inline = Inline::text(text);
    inline.comment_ids.push(comment_id.to_string());
    inline
}

fn tracked_inline(
    text: &str,
    kind: TrackedChangeKind,
    id: &str,
    created_at: DateTime<Utc>,
) -> Inline {
    let mut inline = Inline::text(text);
    inline.tracked_change = Some(TrackedChange {
        id: id.to_string(),
        kind,
        author: "Local User".to_string(),
        created_at,
    });
    inline
}

fn list_item(level: u8, text: &str) -> ListItem {
    ListItem {
        level,
        blocks: vec![paragraph_block_with_inlines(vec![Inline::text(text)])],
    }
}

fn table_row(cells: Vec<TableCell>) -> TableRow {
    TableRow { cells }
}

fn table_cell(
    text: &str,
    background_color: Option<&str>,
    text_alignment: Option<ParagraphAlignment>,
) -> TableCell {
    TableCell {
        presentation: TableCellPresentation {
            background_color: background_color.map(str::to_string),
            text_alignment,
            border: TableCellBorder::Visible,
        },
        blocks: vec![paragraph_block_with_inlines(vec![Inline::text(text)])],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_fixture_has_no_private_content() {
        let document = multilingual_sample();

        assert!(document.stats().word_count >= 4);
        assert_eq!(document.meta.title, "Generated multilingual sample");

        let compatibility = compatibility_sample();
        let serialized =
            serde_json::to_string(&compatibility).expect("compatibility fixture should serialize");
        assert!(compatibility.stats().word_count >= 40);
        assert!(!serialized.contains("/Users/"));
        assert!(!serialized.contains("/Volumes/"));
        assert!(!serialized.contains("Desktop/"));
        assert!(!serialized.contains('@'));
        assert!(!serialized.contains("example.invalid"));
    }

    #[test]
    fn generated_fixture_json_matches_model_shape() {
        let document = load_generated_multilingual_json();
        let generated: serde_json::Value =
            serde_json::from_str(&multilingual_sample_json()).expect("generated JSON must parse");
        let checked_in: serde_json::Value =
            serde_json::from_str(GENERATED_MULTILINGUAL_JSON).expect("checked-in JSON must parse");

        assert_eq!(document.meta.title, "Generated multilingual sample");
        assert_eq!(document.sections[0].blocks.len(), 4);
        assert_eq!(generated, checked_in);
    }

    #[test]
    fn compatibility_fixture_exports_supported_formats() {
        let document = compatibility_sample();
        let html = word_export::export_html(&document).expect("compatibility HTML should export");
        let print_html = word_export::export_print_html(&document)
            .expect("compatibility print HTML should export");

        assert!(word_odf::write_odt_bytes(&document)
            .expect("compatibility ODT should write")
            .starts_with(b"PK"));
        assert!(word_docx::write_docx_bytes(&document)
            .expect("compatibility DOCX should write")
            .starts_with(b"PK"));
        assert!(word_export::export_txt(&document)
            .expect("compatibility TXT should export")
            .contains("Generated Compatibility Report"));
        assert!(html.contains("Generated Compatibility Report"));
        assert!(html.contains("<figure data-asset=\"compatibility-image.png\""));
        assert!(!html.contains("%;\"\">"));
        assert!(print_html.contains("Generated Compatibility Report"));
        assert!(print_html.contains("<figure data-asset=\"compatibility-image.png\""));
        assert!(!print_html.contains("%;\"\">"));
        assert!(word_export::export_basic_pdf(&document)
            .expect("compatibility PDF should export")
            .starts_with(b"%PDF-"));
    }
}
