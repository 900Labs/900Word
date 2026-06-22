use word_core::{Block, Document, Heading, Inline};

pub fn multilingual_sample() -> Document {
    let mut document = Document::new_untitled();
    document.meta.title = "Generated multilingual sample".to_string();
    document.sections[0].blocks = vec![
        Block::Heading(Heading {
            level: 1,
            inlines: vec![Inline::text("Generated sample")],
        }),
        Block::Paragraph(word_core::Paragraph {
            style: word_core::StyleId::from("body"),
            inlines: vec![Inline::text("Hello offline world.")],
        }),
        Block::Paragraph(word_core::Paragraph {
            style: word_core::StyleId::from("body"),
            inlines: vec![Inline::text("مرحبا بالعالم")],
        }),
        Block::Paragraph(word_core::Paragraph {
            style: word_core::StyleId::from("body"),
            inlines: vec![Inline::text("你好，世界")],
        }),
    ];
    document
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
}
