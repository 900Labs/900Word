use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

const BUNDLED_LANGUAGE_TAG: &str = "en-US";
const BUNDLED_DISPLAY_NAME: &str = "English (United States) bootstrap";
const BUNDLED_LICENSE: &str = "GPL-3.0-or-later";
const BUNDLED_SOURCE: &str = "generated bootstrap Hunspell dictionary";
const BUNDLED_AFF: &str = include_str!("../dictionaries/en_US/en_US.aff");
const BUNDLED_DIC: &str = include_str!("../dictionaries/en_US/en_US.dic");
const MAX_USER_DICTIONARY_BYTES: u64 = 1024 * 1024;
const MAX_PERSONAL_DICTIONARY_BYTES: u64 = 128 * 1024;
const MAX_SUGGESTIONS: usize = 5;

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<String>,
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
        self.check_with_personal_words(text, &BTreeSet::new())
    }

    pub fn check_with_personal_words(
        &self,
        text: &str,
        personal_words: &BTreeSet<String>,
    ) -> Vec<SpellIssue> {
        let mut issues = Vec::new();
        let mut word_start = None;

        for (index, ch) in text.char_indices() {
            if is_word_char(ch) {
                word_start.get_or_insert(index);
                continue;
            }

            if let Some(start) = word_start.take() {
                self.push_issue_if_needed(text, start, index, personal_words, &mut issues);
            }
        }

        if let Some(start) = word_start {
            self.push_issue_if_needed(text, start, text.len(), personal_words, &mut issues);
        }

        issues
    }

    fn push_issue_if_needed(
        &self,
        text: &str,
        start: usize,
        end: usize,
        personal_words: &BTreeSet<String>,
        issues: &mut Vec<SpellIssue>,
    ) {
        let word = &text[start..end];
        let normalized = normalize_word(word);
        if !self.words.contains(&normalized) && !personal_words.contains(&normalized) {
            issues.push(SpellIssue {
                word: word.to_string(),
                byte_start: start,
                byte_end: end,
                suggestions: self.suggest(word),
            });
        }
    }

    pub fn suggest(&self, word: &str) -> Vec<String> {
        let normalized = normalize_word(word);
        if normalized.is_empty() {
            return Vec::new();
        }

        let Some(first_char) = normalized.chars().next() else {
            return Vec::new();
        };

        let mut suggestions = Vec::new();
        for candidate in &self.words {
            if suggestions.len() >= MAX_SUGGESTIONS {
                break;
            }
            let length_delta = candidate
                .chars()
                .count()
                .abs_diff(normalized.chars().count());
            if length_delta > 2 {
                continue;
            }
            if candidate.starts_with(first_char)
                && bounded_edit_distance(candidate, &normalized, 2).is_some()
            {
                suggestions.push(candidate.clone());
            }
        }
        suggestions
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

pub fn read_personal_words(
    user_root: &Path,
    language_tag: &str,
) -> Result<BTreeSet<String>, SpellError> {
    let path = personal_words_path(user_root, language_tag)?;
    if !path.exists() {
        return Ok(BTreeSet::new());
    }
    let metadata = fs::symlink_metadata(&path).map_err(|_| SpellError::DictionaryIo)?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_PERSONAL_DICTIONARY_BYTES {
        return Err(SpellError::InvalidDictionary {
            reason: "personal dictionary file is invalid",
        });
    }
    let text = fs::read_to_string(path).map_err(|_| SpellError::DictionaryIo)?;
    Ok(text
        .lines()
        .filter_map(normalize_personal_word)
        .collect::<BTreeSet<_>>())
}

pub fn list_personal_words(
    user_root: &Path,
    language_tag: &str,
) -> Result<Vec<String>, SpellError> {
    Ok(read_personal_words(user_root, language_tag)?
        .into_iter()
        .collect())
}

pub fn add_personal_word(
    user_root: &Path,
    language_tag: &str,
    word: &str,
) -> Result<(), SpellError> {
    fs::create_dir_all(user_root).map_err(|_| SpellError::DictionaryIo)?;
    let normalized = normalize_personal_word(word).ok_or(SpellError::InvalidDictionary {
        reason: "personal dictionary word is invalid",
    })?;
    let path = personal_words_path(user_root, language_tag)?;
    let mut words = read_personal_words(user_root, language_tag)?;
    if !words.insert(normalized) {
        return Ok(());
    }
    let byte_len: usize = words.iter().map(|entry| entry.len() + 1).sum();
    if byte_len as u64 > MAX_PERSONAL_DICTIONARY_BYTES {
        return Err(SpellError::InvalidDictionary {
            reason: "personal dictionary file is too large",
        });
    }
    let mut output = String::new();
    for entry in words {
        output.push_str(&entry);
        output.push('\n');
    }
    fs::write(&path, output).map_err(|_| SpellError::DictionaryIo)?;
    set_private_file_permissions(&path)?;
    Ok(())
}

pub fn remove_personal_word(
    user_root: &Path,
    language_tag: &str,
    word: &str,
) -> Result<Vec<String>, SpellError> {
    let normalized = normalize_personal_word(word).ok_or(SpellError::InvalidDictionary {
        reason: "personal dictionary word is invalid",
    })?;
    let mut words = read_personal_words(user_root, language_tag)?;
    if !words.remove(&normalized) {
        return Ok(words.into_iter().collect());
    }
    write_personal_words(user_root, language_tag, &words)
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

pub fn install_user_dictionary(
    user_root: &Path,
    language_tag: &str,
    aff_source: &Path,
    dic_source: &Path,
) -> Result<DictionaryInfo, SpellError> {
    let language_tag = language_tag.trim();
    validate_language_tag(language_tag)?;
    ensure_private_dictionary_root(user_root)?;
    validate_dictionary_source_path(aff_source, "aff")?;
    validate_dictionary_source_path(dic_source, "dic")?;
    reject_same_source_file(aff_source, dic_source)?;

    let normalized = normalize_language_tag(language_tag);
    let target_aff = matching_aff_path(user_root, &normalized);
    let target_dic = matching_dic_path(user_root, &normalized);
    reject_source_target_overlap(aff_source, &target_aff)?;
    reject_source_target_overlap(dic_source, &target_dic)?;

    let temp_aff = install_temp_path(user_root, &normalized, "aff");
    let temp_dic = install_temp_path(user_root, &normalized, "dic");
    let backup_aff = install_temp_path(user_root, &normalized, "aff-backup");
    let backup_dic = install_temp_path(user_root, &normalized, "dic-backup");
    let install_result = (|| {
        copy_dictionary_source(aff_source, &temp_aff)?;
        copy_dictionary_source(dic_source, &temp_dic)?;

        let aff = read_limited_dictionary_file(&temp_aff)?;
        let dic = read_limited_dictionary_file(&temp_dic)?;
        SpellChecker::from_hunspell_parts(
            &normalized,
            &format!("User dictionary ({normalized})"),
            &aff,
            &dic,
        )?;

        commit_dictionary_install(
            &temp_aff,
            &temp_dic,
            &target_aff,
            &target_dic,
            &backup_aff,
            &backup_dic,
        )?;

        Ok(DictionaryInfo {
            language_tag: normalized.clone(),
            display_name: format!("User dictionary ({normalized})"),
            bundled: false,
            user: true,
            license: "User-provided; verify before distribution".to_string(),
            source: "user dictionary folder".to_string(),
        })
    })();

    if install_result.is_err() {
        let _ = fs::remove_file(&temp_aff);
        let _ = fs::remove_file(&temp_dic);
        let _ = fs::remove_file(&backup_aff);
        let _ = fs::remove_file(&backup_dic);
    }
    install_result
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

fn ensure_private_dictionary_root(user_root: &Path) -> Result<(), SpellError> {
    fs::create_dir_all(user_root).map_err(|_| SpellError::DictionaryIo)?;
    let metadata = fs::symlink_metadata(user_root).map_err(|_| SpellError::DictionaryIo)?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(SpellError::InvalidDictionary {
            reason: "dictionary directory is invalid",
        });
    }
    set_private_directory_permissions(user_root)
}

fn validate_dictionary_source_path(
    path: &Path,
    expected_extension: &str,
) -> Result<(), SpellError> {
    if path.as_os_str().is_empty()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err(SpellError::InvalidDictionary {
            reason: "dictionary source file is unsupported",
        });
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension != expected_extension {
        return Err(SpellError::InvalidDictionary {
            reason: "dictionary source file is unsupported",
        });
    }

    let metadata = fs::symlink_metadata(path).map_err(|_| SpellError::DictionaryIo)?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(SpellError::InvalidDictionary {
            reason: "dictionary source file is unsupported",
        });
    }
    if metadata.len() > MAX_USER_DICTIONARY_BYTES {
        return Err(SpellError::InvalidDictionary {
            reason: "dictionary file is too large",
        });
    }
    fs::read_to_string(path).map_err(|_| SpellError::DictionaryIo)?;
    Ok(())
}

fn reject_same_source_file(left: &Path, right: &Path) -> Result<(), SpellError> {
    let left = fs::canonicalize(left).map_err(|_| SpellError::DictionaryIo)?;
    let right = fs::canonicalize(right).map_err(|_| SpellError::DictionaryIo)?;
    if left == right {
        return Err(SpellError::InvalidDictionary {
            reason: "dictionary source file is unsupported",
        });
    }
    Ok(())
}

fn reject_source_target_overlap(source: &Path, target: &Path) -> Result<(), SpellError> {
    if !target.exists() {
        return Ok(());
    }
    let source = fs::canonicalize(source).map_err(|_| SpellError::DictionaryIo)?;
    let target = fs::canonicalize(target).map_err(|_| SpellError::DictionaryIo)?;
    if source == target {
        return Err(SpellError::InvalidDictionary {
            reason: "dictionary source file is unsupported",
        });
    }
    Ok(())
}

fn copy_dictionary_source(source: &Path, target: &Path) -> Result<(), SpellError> {
    fs::copy(source, target).map_err(|_| SpellError::DictionaryIo)?;
    set_private_file_permissions(target)
}

fn commit_dictionary_install(
    temp_aff: &Path,
    temp_dic: &Path,
    target_aff: &Path,
    target_dic: &Path,
    backup_aff: &Path,
    backup_dic: &Path,
) -> Result<(), SpellError> {
    let had_aff = target_aff.exists();
    let had_dic = target_dic.exists();

    if had_aff {
        fs::rename(target_aff, backup_aff).map_err(|_| SpellError::DictionaryIo)?;
    }
    if had_dic && fs::rename(target_dic, backup_dic).is_err() {
        restore_dictionary_install_targets(
            target_aff, target_dic, backup_aff, backup_dic, had_aff, false,
        );
        return Err(SpellError::DictionaryIo);
    }

    if fs::rename(temp_aff, target_aff).is_err() {
        restore_dictionary_install_targets(
            target_aff, target_dic, backup_aff, backup_dic, had_aff, had_dic,
        );
        return Err(SpellError::DictionaryIo);
    }
    if fs::rename(temp_dic, target_dic).is_err() {
        restore_dictionary_install_targets(
            target_aff, target_dic, backup_aff, backup_dic, had_aff, had_dic,
        );
        return Err(SpellError::DictionaryIo);
    }

    cleanup_dictionary_install_backups(backup_aff, backup_dic);
    Ok(())
}

fn restore_dictionary_install_targets(
    target_aff: &Path,
    target_dic: &Path,
    backup_aff: &Path,
    backup_dic: &Path,
    had_aff: bool,
    had_dic: bool,
) {
    let _ = fs::remove_file(target_aff);
    let _ = fs::remove_file(target_dic);
    if had_aff {
        let _ = fs::rename(backup_aff, target_aff);
    }
    if had_dic {
        let _ = fs::rename(backup_dic, target_dic);
    }
}

fn cleanup_dictionary_install_backups(backup_aff: &Path, backup_dic: &Path) {
    let _ = fs::remove_file(backup_aff);
    let _ = fs::remove_file(backup_dic);
}

fn install_temp_path(user_root: &Path, language_tag: &str, kind: &str) -> PathBuf {
    user_root.join(format!(
        ".900word-dictionary-install-{language_tag}-{kind}-{}-{}.tmp",
        std::process::id(),
        current_unix_nanos()
    ))
}

fn current_unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
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

fn personal_words_path(root: &Path, language_tag: &str) -> Result<PathBuf, SpellError> {
    validate_language_tag(language_tag)?;
    let normalized = normalize_language_tag(language_tag).replace('-', "_");
    Ok(root.join(format!("personal-{normalized}.txt")))
}

fn write_personal_words(
    user_root: &Path,
    language_tag: &str,
    words: &BTreeSet<String>,
) -> Result<Vec<String>, SpellError> {
    let path = personal_words_path(user_root, language_tag)?;
    if words.is_empty() {
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(SpellError::DictionaryIo),
        }
        return Ok(Vec::new());
    }

    let byte_len: usize = words.iter().map(|entry| entry.len() + 1).sum();
    if byte_len as u64 > MAX_PERSONAL_DICTIONARY_BYTES {
        return Err(SpellError::InvalidDictionary {
            reason: "personal dictionary file is too large",
        });
    }
    let mut output = String::new();
    for entry in words {
        output.push_str(entry);
        output.push('\n');
    }
    fs::write(&path, output).map_err(|_| SpellError::DictionaryIo)?;
    set_private_file_permissions(&path)?;
    Ok(words.iter().cloned().collect())
}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path) -> Result<(), SpellError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| SpellError::DictionaryIo)
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path) -> Result<(), SpellError> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> Result<(), SpellError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|_| SpellError::DictionaryIo)
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> Result<(), SpellError> {
    Ok(())
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

fn normalize_personal_word(word: &str) -> Option<String> {
    let normalized = normalize_word(word);
    if normalized.is_empty()
        || normalized.chars().count() > 64
        || !normalized.chars().all(is_word_char)
    {
        return None;
    }
    Some(normalized)
}

fn bounded_edit_distance(left: &str, right: &str, max_distance: usize) -> Option<usize> {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    if left.len().abs_diff(right.len()) > max_distance {
        return None;
    }

    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0; right.len() + 1];
    for (left_index, left_char) in left.iter().enumerate() {
        current[0] = left_index + 1;
        let mut row_min = current[0];
        for (right_index, right_char) in right.iter().enumerate() {
            let substitution = previous[right_index] + usize::from(left_char != right_char);
            let insertion = current[right_index] + 1;
            let deletion = previous[right_index + 1] + 1;
            current[right_index + 1] = substitution.min(insertion).min(deletion);
            row_min = row_min.min(current[right_index + 1]);
        }
        if row_min > max_distance {
            return None;
        }
        std::mem::swap(&mut previous, &mut current);
    }

    let distance = previous[right.len()];
    (distance <= max_distance).then_some(distance)
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

    fn regular_files_in(root: &Path) -> Vec<PathBuf> {
        fs::read_dir(root)
            .expect("test directory should be readable")
            .filter_map(|entry| {
                let path = entry
                    .expect("test directory entry should be readable")
                    .path();
                fs::symlink_metadata(&path)
                    .expect("test file metadata should be readable")
                    .file_type()
                    .is_file()
                    .then_some(path)
            })
            .collect()
    }

    fn only_regular_file_in(root: &Path) -> PathBuf {
        let mut files = regular_files_in(root);
        assert_eq!(files.len(), 1);
        files.remove(0)
    }

    #[test]
    fn english_checker_flags_unknown_word() {
        let checker = SpellChecker::bootstrap_english();

        let issues = checker.check("hello qwerty");

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].word, "qwerty");
    }

    #[test]
    fn english_checker_returns_bounded_suggestions() {
        let checker = SpellChecker::bootstrap_english();

        let issues = checker.check("helo");

        assert_eq!(issues.len(), 1);
        assert!(issues[0].suggestions.iter().any(|word| word == "hello"));
        assert!(issues[0].suggestions.len() <= MAX_SUGGESTIONS);
    }

    #[test]
    fn personal_words_are_used_as_local_overrides() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        add_personal_word(dir.path(), "en-US", "qwerty").expect("personal word should write");

        let words = read_personal_words(dir.path(), "en-US").expect("personal words should read");
        let checker = SpellChecker::bootstrap_english();

        assert!(checker
            .check_with_personal_words("hello qwerty", &words)
            .is_empty());
    }

    #[test]
    fn personal_words_list_is_normalized_and_sorted() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        add_personal_word(dir.path(), "en-US", "Qwerty").expect("personal word should write");
        add_personal_word(dir.path(), "en-US", "Alpha").expect("personal word should write");

        let words =
            list_personal_words(dir.path(), "en-US").expect("personal words should be listed");

        assert_eq!(words, vec!["alpha".to_string(), "qwerty".to_string()]);
    }

    #[test]
    fn missing_personal_dictionary_lists_empty_words() {
        let dir = tempfile::tempdir().expect("temp dir should exist");

        let words = list_personal_words(dir.path(), "en-US").expect("missing list should be empty");

        assert!(words.is_empty());
    }

    #[test]
    fn removing_personal_word_updates_the_local_list() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        add_personal_word(dir.path(), "en-US", "qwerty").expect("personal word should write");
        add_personal_word(dir.path(), "en-US", "zebra").expect("personal word should write");

        let remaining = remove_personal_word(dir.path(), "en-US", "qwerty")
            .expect("personal word should remove");

        assert_eq!(remaining, vec!["zebra".to_string()]);
        assert_eq!(
            list_personal_words(dir.path(), "en-US").expect("personal words should list"),
            vec!["zebra".to_string()]
        );
    }

    #[test]
    fn removing_from_missing_personal_dictionary_is_empty() {
        let dir = tempfile::tempdir().expect("temp dir should exist");

        let remaining = remove_personal_word(dir.path(), "en-US", "qwerty")
            .expect("missing personal dictionary should be a no-op");

        assert!(remaining.is_empty());
    }

    #[test]
    fn invalid_personal_word_remove_is_rejected_without_private_details() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let invalid_word = "!";

        let err = remove_personal_word(dir.path(), "en-US", invalid_word)
            .expect_err("invalid personal word should fail");

        assert_eq!(
            err,
            SpellError::InvalidDictionary {
                reason: "personal dictionary word is invalid"
            }
        );
        assert!(!err.to_string().contains(invalid_word));
        assert!(!err
            .to_string()
            .contains(dir.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn removing_last_personal_word_deletes_the_private_file() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        add_personal_word(dir.path(), "en-US", "qwerty").expect("personal word should write");

        let remaining = remove_personal_word(dir.path(), "en-US", "qwerty")
            .expect("personal word should remove");

        assert!(remaining.is_empty());
        assert!(regular_files_in(dir.path()).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn personal_dictionary_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("temp dir should exist");
        add_personal_word(dir.path(), "en-US", "qwerty").expect("personal word should write");

        let mode = fs::metadata(only_regular_file_in(dir.path()))
            .expect("personal dictionary file should exist")
            .permissions()
            .mode()
            & 0o777;

        assert_eq!(mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn personal_dictionary_remove_rewrite_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("temp dir should exist");
        add_personal_word(dir.path(), "en-US", "qwerty").expect("personal word should write");
        add_personal_word(dir.path(), "en-US", "zebra").expect("personal word should write");
        let path = only_regular_file_in(dir.path());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644))
            .expect("test permissions should apply");

        remove_personal_word(dir.path(), "en-US", "qwerty").expect("personal word should remove");

        let mode = fs::metadata(&path)
            .expect("personal dictionary file should exist")
            .permissions()
            .mode()
            & 0o777;

        assert_eq!(mode, 0o600);
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
    fn install_user_dictionary_copies_lists_and_loads_local_pair() {
        let user_dir = tempfile::tempdir().expect("user dir should exist");
        let source_dir = tempfile::tempdir().expect("source dir should exist");
        write_user_dictionary(source_dir.path(), "sv-SE", &["hej", "dokument"])
            .expect("source dictionary should write");

        let installed = install_user_dictionary(
            user_dir.path(),
            "sv_SE",
            &source_dir.path().join("sv-SE.aff"),
            &source_dir.path().join("sv-SE.dic"),
        )
        .expect("dictionary should install");

        assert_eq!(installed.language_tag, "sv-SE");
        assert!(installed.user);
        assert!(user_dir.path().join("sv-SE.aff").is_file());
        assert!(user_dir.path().join("sv-SE.dic").is_file());
        let dictionaries = list_dictionaries_with_user_root(user_dir.path());
        assert!(dictionaries
            .iter()
            .any(|dictionary| dictionary.language_tag == "sv-SE" && dictionary.user));
        let checker = checker_for_with_user_root("sv-SE", user_dir.path())
            .expect("installed dictionary should load");
        assert!(checker.check("hej dokument").is_empty());
    }

    #[test]
    fn install_user_dictionary_replaces_existing_pair_cleanly() {
        let user_dir = tempfile::tempdir().expect("user dir should exist");
        let source_dir = tempfile::tempdir().expect("source dir should exist");
        write_user_dictionary(user_dir.path(), "sv-SE", &["gammal"])
            .expect("existing dictionary should write");
        write_user_dictionary(source_dir.path(), "sv-SE", &["ny", "ordlista"])
            .expect("source dictionary should write");

        install_user_dictionary(
            user_dir.path(),
            "sv-SE",
            &source_dir.path().join("sv-SE.aff"),
            &source_dir.path().join("sv-SE.dic"),
        )
        .expect("replacement dictionary should install");

        let checker = checker_for_with_user_root("sv-SE", user_dir.path())
            .expect("replacement dictionary should load");
        assert!(checker.check("ny ordlista").is_empty());
        assert_eq!(checker.check("gammal").len(), 1);
        assert!(regular_files_in(user_dir.path()).iter().all(|path| !path
            .to_string_lossy()
            .contains(".900word-dictionary-install")));
    }

    #[test]
    fn install_user_dictionary_rejects_invalid_language() {
        let user_dir = tempfile::tempdir().expect("user dir should exist");
        let source_dir = tempfile::tempdir().expect("source dir should exist");
        write_user_dictionary(source_dir.path(), "sv-SE", &["hej"])
            .expect("source dictionary should write");

        let err = install_user_dictionary(
            user_dir.path(),
            "privateclient",
            &source_dir.path().join("sv-SE.aff"),
            &source_dir.path().join("sv-SE.dic"),
        )
        .expect_err("invalid language should fail");

        assert_eq!(
            err,
            SpellError::InvalidDictionary {
                reason: "language tag is invalid"
            }
        );
        assert!(regular_files_in(user_dir.path()).is_empty());
    }

    #[test]
    fn install_user_dictionary_rejects_wrong_extension() {
        let user_dir = tempfile::tempdir().expect("user dir should exist");
        let source_dir = tempfile::tempdir().expect("source dir should exist");
        fs::write(source_dir.path().join("sv-SE.txt"), "SET UTF-8\n").expect("aff should write");
        fs::write(source_dir.path().join("sv-SE.dic"), "1\nhej\n").expect("dic should write");

        let err = install_user_dictionary(
            user_dir.path(),
            "sv-SE",
            &source_dir.path().join("sv-SE.txt"),
            &source_dir.path().join("sv-SE.dic"),
        )
        .expect_err("wrong extension should fail");

        assert_eq!(
            err,
            SpellError::InvalidDictionary {
                reason: "dictionary source file is unsupported"
            }
        );
        assert!(regular_files_in(user_dir.path()).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn install_user_dictionary_rejects_symlink_source() {
        use std::os::unix::fs::symlink;

        let user_dir = tempfile::tempdir().expect("user dir should exist");
        let source_dir = tempfile::tempdir().expect("source dir should exist");
        write_user_dictionary(source_dir.path(), "sv-SE", &["hej"])
            .expect("source dictionary should write");
        symlink(
            source_dir.path().join("sv-SE.aff"),
            source_dir.path().join("linked.aff"),
        )
        .expect("symlink should write");

        let err = install_user_dictionary(
            user_dir.path(),
            "sv-SE",
            &source_dir.path().join("linked.aff"),
            &source_dir.path().join("sv-SE.dic"),
        )
        .expect_err("symlink source should fail");

        assert_eq!(
            err,
            SpellError::InvalidDictionary {
                reason: "dictionary source file is unsupported"
            }
        );
        assert!(regular_files_in(user_dir.path()).is_empty());
    }

    #[test]
    fn install_user_dictionary_rejects_oversized_source() {
        let user_dir = tempfile::tempdir().expect("user dir should exist");
        let source_dir = tempfile::tempdir().expect("source dir should exist");
        fs::write(source_dir.path().join("sv-SE.aff"), "SET UTF-8\n").expect("aff should write");
        fs::write(
            source_dir.path().join("sv-SE.dic"),
            "a".repeat(MAX_USER_DICTIONARY_BYTES as usize + 1),
        )
        .expect("oversized dic should write");

        let err = install_user_dictionary(
            user_dir.path(),
            "sv-SE",
            &source_dir.path().join("sv-SE.aff"),
            &source_dir.path().join("sv-SE.dic"),
        )
        .expect_err("oversized source should fail");

        assert_eq!(
            err,
            SpellError::InvalidDictionary {
                reason: "dictionary file is too large"
            }
        );
        assert!(regular_files_in(user_dir.path()).is_empty());
    }

    #[test]
    fn install_user_dictionary_cleans_up_failed_validation() {
        let user_dir = tempfile::tempdir().expect("user dir should exist");
        let source_dir = tempfile::tempdir().expect("source dir should exist");
        fs::write(source_dir.path().join("sv-SE.aff"), "SET UTF-8\n").expect("aff should write");
        fs::write(source_dir.path().join("sv-SE.dic"), "0\n").expect("dic should write");

        let err = install_user_dictionary(
            user_dir.path(),
            "sv-SE",
            &source_dir.path().join("sv-SE.aff"),
            &source_dir.path().join("sv-SE.dic"),
        )
        .expect_err("empty dictionary should fail validation");

        assert_eq!(
            err,
            SpellError::InvalidDictionary {
                reason: "dictionary word list is empty"
            }
        );
        assert!(regular_files_in(user_dir.path()).is_empty());
        assert!(list_user_dictionaries(user_dir.path()).is_empty());
    }

    #[test]
    fn install_user_dictionary_restores_existing_pair_when_second_target_replace_fails() {
        let user_dir = tempfile::tempdir().expect("user dir should exist");
        write_user_dictionary(user_dir.path(), "sv-SE", &["gammal"])
            .expect("existing dictionary should write");
        let temp_aff = user_dir.path().join("pending.aff.tmp");
        let missing_temp_dic = user_dir.path().join("missing.dic.tmp");
        let target_aff = user_dir.path().join("sv-SE.aff");
        let target_dic = user_dir.path().join("sv-SE.dic");
        let backup_aff = user_dir.path().join("backup.aff.tmp");
        let backup_dic = user_dir.path().join("backup.dic.tmp");
        fs::write(&temp_aff, "SET UTF-8\n").expect("pending aff should write");

        let err = commit_dictionary_install(
            &temp_aff,
            &missing_temp_dic,
            &target_aff,
            &target_dic,
            &backup_aff,
            &backup_dic,
        )
        .expect_err("missing second temp file should fail");

        assert_eq!(err, SpellError::DictionaryIo);
        assert!(!fs::read_to_string(&target_aff)
            .expect("restored aff should be readable")
            .contains("pending"));
        let checker = checker_for_with_user_root("sv-SE", user_dir.path())
            .expect("existing dictionary should remain usable");
        assert!(checker.check("gammal").is_empty());
        assert!(!backup_aff.exists());
        assert!(!backup_dic.exists());
    }

    #[cfg(unix)]
    #[test]
    fn installed_user_dictionary_files_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let user_dir = tempfile::tempdir().expect("user dir should exist");
        let source_dir = tempfile::tempdir().expect("source dir should exist");
        write_user_dictionary(source_dir.path(), "sv-SE", &["hej"])
            .expect("source dictionary should write");

        install_user_dictionary(
            user_dir.path(),
            "sv-SE",
            &source_dir.path().join("sv-SE.aff"),
            &source_dir.path().join("sv-SE.dic"),
        )
        .expect("dictionary should install");

        let dir_mode = fs::metadata(user_dir.path())
            .expect("dictionary dir metadata should exist")
            .permissions()
            .mode()
            & 0o777;
        let aff_mode = fs::metadata(user_dir.path().join("sv-SE.aff"))
            .expect("aff metadata should exist")
            .permissions()
            .mode()
            & 0o777;
        let dic_mode = fs::metadata(user_dir.path().join("sv-SE.dic"))
            .expect("dic metadata should exist")
            .permissions()
            .mode()
            & 0o777;

        assert_eq!(dir_mode, 0o700);
        assert_eq!(aff_mode, 0o600);
        assert_eq!(dic_mode, 0o600);
    }

    #[test]
    fn install_user_dictionary_error_text_stays_private() {
        let user_dir = tempfile::tempdir().expect("user dir should exist");
        let source_dir = tempfile::tempdir().expect("source dir should exist");
        fs::write(source_dir.path().join("private-client.aff"), "SET UTF-8\n")
            .expect("aff should write");
        fs::write(source_dir.path().join("private-client.dic"), "0\n").expect("dic should write");

        let err = install_user_dictionary(
            user_dir.path(),
            "sv-SE",
            &source_dir.path().join("private-client.aff"),
            &source_dir.path().join("private-client.dic"),
        )
        .expect_err("invalid dictionary should fail");
        let error = err.to_string();

        assert!(!error.contains(user_dir.path().to_string_lossy().as_ref()));
        assert!(!error.contains(source_dir.path().to_string_lossy().as_ref()));
        assert!(!error.contains("private-client"));
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
