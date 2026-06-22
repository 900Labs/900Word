use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictionaryInfo {
    pub language_tag: String,
    pub display_name: String,
    pub bundled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellIssue {
    pub word: String,
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SpellError {
    #[error("dictionary is not available for {language_tag}")]
    MissingDictionary { language_tag: String },
}

#[derive(Debug, Clone)]
pub struct SpellChecker {
    language_tag: String,
    words: BTreeSet<String>,
}

impl SpellChecker {
    pub fn bootstrap_english() -> Self {
        let words = [
            "a", "and", "document", "draft", "for", "hello", "local", "offline", "start", "the",
            "word", "writing",
        ];
        Self {
            language_tag: "en".to_string(),
            words: words.into_iter().map(str::to_string).collect(),
        }
    }

    pub fn language_tag(&self) -> &str {
        &self.language_tag
    }

    pub fn check(&self, text: &str) -> Vec<SpellIssue> {
        let mut issues = Vec::new();
        let mut word_start = None;

        for (index, ch) in text.char_indices() {
            if ch.is_alphabetic() {
                word_start.get_or_insert(index);
                continue;
            }

            if let Some(start) = word_start.take() {
                self.push_issue_if_needed(text, start, index, &mut issues);
            }
        }

        if let Some(start) = word_start {
            self.push_issue_if_needed(text, start, text.len(), &mut issues);
        }

        issues
    }

    fn push_issue_if_needed(
        &self,
        text: &str,
        start: usize,
        end: usize,
        issues: &mut Vec<SpellIssue>,
    ) {
        let word = &text[start..end];
        let normalized = word.to_lowercase();
        if !self.words.contains(&normalized) {
            issues.push(SpellIssue {
                word: word.to_string(),
                byte_start: start,
                byte_end: end,
            });
        }
    }
}

pub fn list_dictionaries() -> Vec<DictionaryInfo> {
    vec![DictionaryInfo {
        language_tag: "en".to_string(),
        display_name: "English bootstrap dictionary".to_string(),
        bundled: true,
    }]
}

pub fn checker_for(language_tag: &str) -> Result<SpellChecker, SpellError> {
    match language_tag {
        "en" => Ok(SpellChecker::bootstrap_english()),
        _ => Err(SpellError::MissingDictionary {
            language_tag: language_tag.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_checker_flags_unknown_word() {
        let checker = SpellChecker::bootstrap_english();

        let issues = checker.check("hello qwerty");

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].word, "qwerty");
    }

    #[test]
    fn missing_dictionary_is_explicit() {
        let err = checker_for("zz").expect_err("unknown language should fail");

        assert_eq!(
            err,
            SpellError::MissingDictionary {
                language_tag: "zz".to_string()
            }
        );
    }
}
