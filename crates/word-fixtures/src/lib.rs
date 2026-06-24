use chrono::{DateTime, Utc};
use uuid::Uuid;
use word_core::{Block, Document, Heading, Inline};

pub const GENERATED_MULTILINGUAL_JSON: &str =
    include_str!("../fixtures/generated-multilingual.json");

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_fixture_has_no_private_content() {
        let document = multilingual_sample();

        assert!(document.stats().word_count >= 4);
        assert_eq!(document.meta.title, "Generated multilingual sample");
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
}
