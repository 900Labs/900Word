use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

const BUNDLED_LANGUAGE_TAG: &str = "en-US";
const BUNDLED_DISPLAY_NAME: &str = "English (United States) bootstrap";
const BUNDLED_LICENSE: &str = "GPL-3.0-or-later";
const BUNDLED_SOURCE: &str = "generated bootstrap Hunspell dictionary";
const BUNDLED_AFF: &str = include_str!("../dictionaries/en_US/en_US.aff");
const BUNDLED_DIC: &str = include_str!("../dictionaries/en_US/en_US.dic");
const MAX_USER_DICTIONARY_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictionaryInfo {
    pub language_tag: String,
    pub display_name: String,
    pub bundled: bool,
    pub user: bool,
    pub license: String,
    pub source: String,
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
    #[error("dictionary is invalid: {reason}")]
    InvalidDictionary { reason: &'static str },
    #[error("dictionary file operation failed")]
    DictionaryIo,
}

#[derive(Debug, Clone)]
pub struct SpellChecker {
    language_tag: String,
    display_name: String,
    words: BTreeSet<String>,
}

impl SpellChecker {
    pub fn bootstrap_english() -> Self {
        bundled_checker().expect("bundled bootstrap dictionary must be valid")
    }

    pub fn from_hunspell_parts(
        language_tag: &str,
        display_name: &str,
        aff: &str,
        dic: &str,
    ) -> Result<Self, SpellError> {
        validate_language_tag(language_tag)?;
        validate_aff(aff)?;
        let words = parse_hunspell_dic(dic)?;
        if words.is_empty() {
            return Err(SpellError::InvalidDictionary {
                reason: "dictionary word list is empty",
            });
        }
        Ok(Self {
            language_tag: normalize_language_tag(language_tag),
            display_name: display_name.to_string(),
            words,
        })
    }

    pub fn language_tag(&self) -> &str {
        &self.language_tag
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn check(&self, text: &str) -> Vec<SpellIssue> {
        let mut issues = Vec::new();
        let mut word_start = None;

        for (index, ch) in text.char_indices() {
            if is_word_char(ch) {
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
        let normalized = normalize_word(word);
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
    list_bundled_dictionaries()
}

pub fn list_bundled_dictionaries() -> Vec<DictionaryInfo> {
    vec![bundled_dictionary_info()]
}

pub fn list_dictionaries_with_user_root(user_root: &Path) -> Vec<DictionaryInfo> {
    let mut dictionaries = list_bundled_dictionaries();
    dictionaries.extend(list_user_dictionaries(user_root));
    dictionaries
}

pub fn checker_for(language_tag: &str) -> Result<SpellChecker, SpellError> {
    let normalized = normalize_language_tag(language_tag);
    match normalized.as_str() {
        "en" | BUNDLED_LANGUAGE_TAG => bundled_checker(),
        _ => Err(SpellError::MissingDictionary {
            language_tag: language_tag.to_string(),
        }),
    }
}

pub fn checker_for_with_user_root(
    language_tag: &str,
    user_root: &Path,
) -> Result<SpellChecker, SpellError> {
    if let Some(checker) = load_user_checker(language_tag, user_root)? {
        return Ok(checker);
    }
    checker_for(language_tag)
}

pub fn list_user_dictionaries(user_root: &Path) -> Vec<DictionaryInfo> {
    let Ok(entries) = fs::read_dir(user_root) else {
        return Vec::new();
    };

    let mut dictionaries = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("dic") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if !is_regular_dictionary_file(&path) {
            continue;
        }
        if validate_language_tag(stem).is_err() {
            continue;
        }
        if !is_regular_dictionary_file(&matching_aff_path(user_root, stem)) {
            continue;
        }
        let tag = normalize_language_tag(stem);
        if dictionaries
            .iter()
            .any(|dictionary: &DictionaryInfo| dictionary.language_tag == tag)
        {
            continue;
        }
        dictionaries.push(DictionaryInfo {
            language_tag: tag.clone(),
            display_name: format!("User dictionary ({tag})"),
            bundled: false,
            user: true,
            license: "User-provided; verify before distribution".to_string(),
            source: "user dictionary folder".to_string(),
        });
    }
    dictionaries.sort_by(|left, right| left.language_tag.cmp(&right.language_tag));
    dictionaries
}

pub fn user_dictionary_template(language_tag: &str) -> Result<(String, String), SpellError> {
    validate_language_tag(language_tag)?;
    Ok((
        format!("SET UTF-8\n# 900Word user dictionary for {language_tag}\n"),
        "1\nexample\n".to_string(),
    ))
}

fn bundled_checker() -> Result<SpellChecker, SpellError> {
    SpellChecker::from_hunspell_parts(
        BUNDLED_LANGUAGE_TAG,
        BUNDLED_DISPLAY_NAME,
        BUNDLED_AFF,
        BUNDLED_DIC,
    )
}

fn bundled_dictionary_info() -> DictionaryInfo {
    DictionaryInfo {
        language_tag: BUNDLED_LANGUAGE_TAG.to_string(),
        display_name: BUNDLED_DISPLAY_NAME.to_string(),
        bundled: true,
        user: false,
        license: BUNDLED_LICENSE.to_string(),
        source: BUNDLED_SOURCE.to_string(),
    }
}

fn load_user_checker(
    language_tag: &str,
    user_root: &Path,
) -> Result<Option<SpellChecker>, SpellError> {
    validate_language_tag(language_tag)?;
    let normalized = normalize_language_tag(language_tag);
    for stem in dictionary_file_stems(&normalized) {
        let dic_path = matching_dic_path(user_root, &stem);
        let aff_path = matching_aff_path(user_root, &stem);
        if !is_regular_dictionary_file(&dic_path) || !is_regular_dictionary_file(&aff_path) {
            continue;
        }

        let aff = read_limited_dictionary_file(&aff_path)?;
        let dic = read_limited_dictionary_file(&dic_path)?;
        return SpellChecker::from_hunspell_parts(
            &normalized,
            &format!("User dictionary ({normalized})"),
            &aff,
            &dic,
        )
        .map(Some);
    }
    Ok(None)
}

fn read_limited_dictionary_file(path: &Path) -> Result<String, SpellError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| SpellError::DictionaryIo)?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_USER_DICTIONARY_BYTES {
        return Err(SpellError::InvalidDictionary {
            reason: "dictionary file is too large",
        });
    }
    fs::read_to_string(path).map_err(|_| SpellError::DictionaryIo)
}

fn is_regular_dictionary_file(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| {
            metadata.file_type().is_file() && metadata.len() <= MAX_USER_DICTIONARY_BYTES
        })
        .unwrap_or(false)
}

fn matching_dic_path(root: &Path, language_tag: &str) -> PathBuf {
    root.join(format!("{language_tag}.dic"))
}

fn matching_aff_path(root: &Path, language_tag: &str) -> PathBuf {
    root.join(format!("{language_tag}.aff"))
}

fn dictionary_file_stems(language_tag: &str) -> Vec<String> {
    let normalized = normalize_language_tag(language_tag);
    let underscore = normalized.replace('-', "_");
    if normalized == underscore {
        vec![normalized]
    } else {
        vec![normalized, underscore]
    }
}

fn validate_language_tag(language_tag: &str) -> Result<(), SpellError> {
    let normalized = normalize_language_tag(language_tag);
    if normalized.is_empty() || normalized.len() > 35 {
        return Err(SpellError::InvalidDictionary {
            reason: "language tag is invalid",
        });
    }
    let mut parts = normalized.split('-');
    let Some(primary) = parts.next() else {
        return Err(SpellError::InvalidDictionary {
            reason: "language tag is invalid",
        });
    };
    if !(2..=3).contains(&primary.len()) || !primary.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        return Err(SpellError::InvalidDictionary {
            reason: "language tag is invalid",
        });
    }
    for part in parts {
        if part.is_empty()
            || part.len() > 8
            || !part.bytes().all(|byte| byte.is_ascii_alphanumeric())
        {
            return Err(SpellError::InvalidDictionary {
                reason: "language tag is invalid",
            });
        }
    }
    Ok(())
}

fn validate_aff(aff: &str) -> Result<(), SpellError> {
    if !aff
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case("SET UTF-8"))
    {
        return Err(SpellError::InvalidDictionary {
            reason: "only UTF-8 Hunspell dictionaries are supported",
        });
    }
    Ok(())
}

fn parse_hunspell_dic(dic: &str) -> Result<BTreeSet<String>, SpellError> {
    let mut lines = dic.lines();
    let Some(first) = lines.next() else {
        return Err(SpellError::InvalidDictionary {
            reason: "dictionary word list is empty",
        });
    };

    let mut words = BTreeSet::new();
    if first
        .trim_start_matches('\u{feff}')
        .trim()
        .parse::<usize>()
        .is_err()
    {
        push_dictionary_word(first, &mut words);
    }

    for line in lines {
        push_dictionary_word(line, &mut words);
    }
    Ok(words)
}

fn push_dictionary_word(line: &str, words: &mut BTreeSet<String>) {
    let candidate = line
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .split('/')
        .next()
        .unwrap_or_default()
        .trim();
    if candidate.is_empty() || candidate.starts_with('#') {
        return;
    }
    words.insert(normalize_word(candidate));
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphabetic() || ch == '\''
}

fn normalize_word(word: &str) -> String {
    word.trim_matches('\'').to_lowercase()
}

fn normalize_language_tag(language_tag: &str) -> String {
    language_tag.replace('_', "-")
}

#[cfg(test)]
fn write_user_dictionary(
    root: &Path,
    language_tag: &str,
    words: &[&str],
) -> Result<(), std::io::Error> {
    fs::create_dir_all(root)?;
    fs::write(
        matching_aff_path(root, language_tag),
        format!("SET UTF-8\n# test dictionary for {language_tag}\n"),
    )?;
    let mut dic = format!("{}\n", words.len());
    for word in words {
        dic.push_str(word);
        dic.push('\n');
    }
    fs::write(matching_dic_path(root, language_tag), dic)
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
    fn bundled_dictionary_has_license_metadata() {
        let dictionaries = list_dictionaries();

        assert_eq!(dictionaries[0].language_tag, "en-US");
        assert_eq!(dictionaries[0].license, "GPL-3.0-or-later");
        assert!(dictionaries[0].bundled);
        assert!(!dictionaries[0].user);
    }

    #[test]
    fn en_alias_uses_bundled_dictionary() {
        let checker = checker_for("en").expect("en alias should load bundled dictionary");

        assert_eq!(checker.language_tag(), "en-US");
    }

    #[test]
    fn user_dictionary_is_listed_and_used() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        write_user_dictionary(dir.path(), "fr-FR", &["bonjour", "document"])
            .expect("test dictionary should write");

        let dictionaries = list_dictionaries_with_user_root(dir.path());
        let user_dictionary = dictionaries
            .iter()
            .find(|dictionary| dictionary.language_tag == "fr-FR")
            .expect("user dictionary should be listed");
        assert!(user_dictionary.user);
        assert_eq!(
            user_dictionary.license,
            "User-provided; verify before distribution"
        );

        let checker =
            checker_for_with_user_root("fr-FR", dir.path()).expect("user dictionary should load");
        assert!(checker.check("bonjour document").is_empty());
        assert_eq!(checker.check("bonjour unknown").len(), 1);
    }

    #[test]
    fn underscore_named_user_dictionary_is_listed_and_used() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        write_user_dictionary(dir.path(), "fr_FR", &["bonjour", "document"])
            .expect("test dictionary should write");

        let dictionaries = list_dictionaries_with_user_root(dir.path());
        assert!(dictionaries
            .iter()
            .any(|dictionary| dictionary.language_tag == "fr-FR"));

        let checker = checker_for_with_user_root("fr-FR", dir.path())
            .expect("underscore dictionary should load from normalized tag");
        assert!(checker.check("bonjour document").is_empty());
    }

    #[test]
    fn incomplete_user_dictionary_is_ignored() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        fs::write(dir.path().join("de-DE.dic"), "1\nhallo\n").expect("dic should write");

        assert!(list_user_dictionaries(dir.path()).is_empty());
    }

    #[test]
    fn user_dictionary_filenames_must_be_language_tags() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        write_user_dictionary(dir.path(), "privateclient", &["private"])
            .expect("test dictionary should write");

        assert!(list_user_dictionaries(dir.path()).is_empty());
        assert!(checker_for_with_user_root("privateclient", dir.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_user_dictionaries_are_rejected() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().expect("temp dir should exist");
        let external = tempfile::tempdir().expect("external temp dir should exist");
        write_user_dictionary(external.path(), "fr-FR", &["bonjour"])
            .expect("external dictionary should write");
        symlink(
            external.path().join("fr-FR.aff"),
            dir.path().join("fr-FR.aff"),
        )
        .expect("aff symlink should write");
        symlink(
            external.path().join("fr-FR.dic"),
            dir.path().join("fr-FR.dic"),
        )
        .expect("dic symlink should write");

        assert!(list_user_dictionaries(dir.path()).is_empty());
        assert!(checker_for_with_user_root("fr-FR", dir.path()).is_err());
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
