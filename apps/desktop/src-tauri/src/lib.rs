use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, State};
use word_core::{
    AssetRef, Block, Document, DocumentCommand, DocumentError, DocumentStats, Heading, ImageBlock,
    ImagePresentation, Inline, ListBlock, ListItem, Paragraph, StyleId, Table, TableCell, TableRow,
    UndoStack,
};
use word_export::{PdfExportOptions, PdfPageRange};
use word_spell::{DictionaryInfo, SpellIssue};

const MAX_DOCUMENT_BYTES: u64 = 32 * 1024 * 1024;
const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;
const IMAGE_TOO_LARGE_ERROR: &str = "image file is too large";
const MAX_RECENT_DOCUMENTS: usize = 5;
const RECOVERY_DIR_NAME: &str = "900word-recovery";
const RECOVERY_SNAPSHOTS_PER_DOCUMENT: usize = 3;
const MAX_RECOVERY_SNAPSHOTS: usize = 20;
const MAX_RECOVERY_TOKEN_LEN: usize = 96;
const USER_DICTIONARY_DIR_NAME: &str = "dictionaries";
const SETTINGS_FILE_NAME: &str = "settings.json";
const MAX_SETTINGS_BYTES: u64 = 64 * 1024;
const FALLBACK_LANGUAGE_TAG: &str = "en-US";

#[derive(Debug)]
struct AppState {
    session: Mutex<DocumentSession>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            session: Mutex::new(DocumentSession::default()),
        }
    }
}

#[derive(Debug)]
struct DocumentSession {
    document: Document,
    undo: UndoStack,
    current_path: Option<PathBuf>,
    dirty: bool,
    recent_documents: Vec<RecentEntry>,
    next_recent_id: u64,
}

impl Default for DocumentSession {
    fn default() -> Self {
        Self {
            document: Document::new_untitled(),
            undo: UndoStack::default(),
            current_path: None,
            dirty: false,
            recent_documents: Vec::new(),
            next_recent_id: 1,
        }
    }
}

#[derive(Debug, Clone)]
struct RecentEntry {
    token: String,
    path: PathBuf,
    label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub telemetry_enabled: bool,
    #[serde(default)]
    pub language_tag: String,
    #[serde(default)]
    pub ui_locale: String,
    #[serde(default)]
    pub high_contrast: bool,
    #[serde(default)]
    pub large_toolbar: bool,
    #[serde(default)]
    pub reduced_motion: bool,
    #[serde(default)]
    pub low_resource: bool,
    #[serde(default)]
    pub smart_typing: SmartTypingSettings,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SmartTypingSettings {
    #[serde(default)]
    pub capitalize_sentences: bool,
    #[serde(default)]
    pub smart_quotes: bool,
    #[serde(default)]
    pub smart_dashes: bool,
    #[serde(default)]
    pub typo_replacements: bool,
    #[serde(default)]
    pub list_triggers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentDocumentSummary {
    pub token: String,
    pub label: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryDocumentSummary {
    pub token: String,
    pub label: String,
    pub modified_unix_seconds: u64,
    pub byte_len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveryTokenParts {
    document_key: String,
}

#[derive(Debug, Clone)]
struct RecoverySnapshotEntry {
    token: String,
    document_key: String,
    path: PathBuf,
    modified_sort_key: u128,
    modified_unix_seconds: u64,
    byte_len: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentFileState {
    pub has_current_path: bool,
    pub dirty: bool,
    pub recent_documents: Vec<RecentDocumentSummary>,
    pub recovery_documents: Vec<RecoveryDocumentSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSummary {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellCheckResult {
    pub language_tag: String,
    pub dictionary_display_name: String,
    pub issues: Vec<SpellIssue>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFileResult {
    pub format: String,
    pub byte_len: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfExportRequest {
    #[serde(default)]
    pub page_start: Option<usize>,
    #[serde(default)]
    pub page_end: Option<usize>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            telemetry_enabled: false,
            language_tag: FALLBACK_LANGUAGE_TAG.to_string(),
            ui_locale: "en-US".to_string(),
            high_contrast: false,
            large_toolbar: false,
            reduced_motion: false,
            low_resource: false,
            smart_typing: SmartTypingSettings::default(),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            new_document,
            new_document_from_template,
            list_templates,
            open_document,
            open_docx_document,
            open_recent_document,
            save_document,
            save_document_as,
            autosave_document,
            get_document_file_state,
            list_recovery_documents,
            recover_document,
            discard_recovery,
            get_document_state,
            apply_document_command,
            import_image,
            undo,
            redo,
            get_document_stats,
            export_txt,
            export_html,
            export_pdf,
            export_txt_to_path,
            export_html_to_path,
            export_pdf_to_path,
            export_docx_to_path,
            prepare_print_html,
            check_spelling,
            add_to_personal_dictionary,
            list_personal_dictionary_words,
            remove_from_personal_dictionary,
            list_dictionaries,
            install_user_dictionary,
            remove_user_dictionary,
            get_settings,
            update_settings,
            reset_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running 900Word");
}

#[tauri::command]
fn new_document(state: State<'_, AppState>) -> Result<Document, String> {
    let mut session = lock_session(&state)?;
    session.document = Document::new_untitled();
    session.undo = UndoStack::default();
    session.current_path = None;
    session.dirty = false;
    Ok(session.document.clone())
}

#[tauri::command]
fn new_document_from_template(
    template_id: String,
    state: State<'_, AppState>,
) -> Result<Document, String> {
    let document = build_template_document(&template_id)?;
    let mut session = lock_session(&state)?;
    session.document = document;
    session.undo = UndoStack::default();
    session.current_path = None;
    session.dirty = false;
    Ok(session.document.clone())
}

#[tauri::command]
fn list_templates() -> Vec<TemplateSummary> {
    template_summaries()
}

#[tauri::command]
fn open_document(path: String, state: State<'_, AppState>) -> Result<Document, String> {
    let path = validate_path(&path, "odt")?;
    let document = read_document_from_path(&path)?;

    let mut session = lock_session(&state)?;
    session.document = document;
    session.undo = UndoStack::default();
    session.current_path = Some(path.clone());
    session.dirty = false;
    remember_recent_path(&mut session, path);
    Ok(session.document.clone())
}

#[tauri::command]
fn open_docx_document(path: String, state: State<'_, AppState>) -> Result<Document, String> {
    let path = validate_path(&path, "docx")?;
    let document = read_docx_document_from_path(&path)?;

    let mut session = lock_session(&state)?;
    session.document = document;
    session.undo = UndoStack::default();
    session.current_path = None;
    session.dirty = true;
    Ok(session.document.clone())
}

#[tauri::command]
fn open_recent_document(token: String, state: State<'_, AppState>) -> Result<Document, String> {
    let path = {
        let session = lock_session(&state)?;
        session
            .recent_documents
            .iter()
            .find(|entry| entry.token == token)
            .map(|entry| entry.path.clone())
            .ok_or_else(|| "recent document is unavailable".to_string())?
    };
    let document = read_document_from_path(&path)?;
    let mut session = lock_session(&state)?;
    session.document = document;
    session.undo = UndoStack::default();
    session.current_path = Some(path.clone());
    session.dirty = false;
    remember_recent_path(&mut session, path);
    Ok(session.document.clone())
}

#[tauri::command]
fn save_document(state: State<'_, AppState>) -> Result<DocumentFileState, String> {
    let mut session = lock_session(&state)?;
    let path = session
        .current_path
        .clone()
        .ok_or_else(|| "no current document path; use save_document_as".to_string())?;
    write_document_to_path(&session.document, &path)?;
    session.dirty = false;
    remember_recent_path(&mut session, path);
    document_file_state_from_session(&session)
}

#[tauri::command]
fn save_document_as(path: String, state: State<'_, AppState>) -> Result<DocumentFileState, String> {
    let path = validate_path(&path, "odt")?;
    let mut session = lock_session(&state)?;
    write_document_to_path(&session.document, &path)?;
    session.current_path = Some(path.clone());
    session.dirty = false;
    remember_recent_path(&mut session, path);
    document_file_state_from_session(&session)
}

#[tauri::command]
fn autosave_document(state: State<'_, AppState>) -> Result<RecoveryDocumentSummary, String> {
    let session = lock_session(&state)?;
    write_recovery_document(&session.document)
}

#[tauri::command]
fn get_document_file_state(state: State<'_, AppState>) -> Result<DocumentFileState, String> {
    let session = lock_session(&state)?;
    document_file_state_from_session(&session)
}

#[tauri::command]
fn list_recovery_documents() -> Result<Vec<RecoveryDocumentSummary>, String> {
    list_recovery_documents_from_disk()
}

#[tauri::command]
fn recover_document(token: String, state: State<'_, AppState>) -> Result<Document, String> {
    let mut session = lock_session(&state)?;
    recover_document_from_dir(&token, &recovery_dir(), &mut session)
}

#[tauri::command]
fn discard_recovery(token: String) -> Result<(), String> {
    discard_recovery_from_dir(&token, &recovery_dir())
}

fn discard_recovery_from_dir(token: &str, dir: &Path) -> Result<(), String> {
    let path = recovery_path_for_token_in_dir(token, dir)?;
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(safe_io_error(error)),
    }
}

#[tauri::command]
fn get_document_state(state: State<'_, AppState>) -> Result<Document, String> {
    let session = lock_session(&state)?;
    Ok(session.document.clone())
}

#[tauri::command]
fn apply_document_command(
    command: DocumentCommand,
    state: State<'_, AppState>,
) -> Result<Document, String> {
    let mut session = lock_session(&state)?;
    let mut document = session.document.clone();
    session
        .undo
        .apply(&mut document, command)
        .map_err(|err: DocumentError| err.to_string())?;
    session.document = document;
    session.dirty = true;
    Ok(session.document.clone())
}

#[tauri::command]
fn import_image(
    path: String,
    section_index: Option<usize>,
    block_index: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Document, String> {
    let path = validate_image_path(&path)?;
    let mut session = lock_session(&state)?;
    import_image_into_session(&mut session, &path, section_index, block_index)
}

#[tauri::command]
fn undo(state: State<'_, AppState>) -> Result<Document, String> {
    let mut session = lock_session(&state)?;
    let mut document = session.document.clone();
    session
        .undo
        .undo(&mut document)
        .map_err(|err: DocumentError| err.to_string())?;
    session.document = document;
    session.dirty = true;
    Ok(session.document.clone())
}

#[tauri::command]
fn redo(state: State<'_, AppState>) -> Result<Document, String> {
    let mut session = lock_session(&state)?;
    let mut document = session.document.clone();
    session
        .undo
        .redo(&mut document)
        .map_err(|err: DocumentError| err.to_string())?;
    session.document = document;
    session.dirty = true;
    Ok(session.document.clone())
}

#[tauri::command]
fn get_document_stats(state: State<'_, AppState>) -> Result<DocumentStats, String> {
    let session = lock_session(&state)?;
    Ok(session.document.stats())
}

#[tauri::command]
fn export_txt(state: State<'_, AppState>) -> Result<String, String> {
    let session = lock_session(&state)?;
    word_export::export_txt(&session.document).map_err(|err| err.to_string())
}

#[tauri::command]
fn export_html(state: State<'_, AppState>) -> Result<String, String> {
    let session = lock_session(&state)?;
    word_export::export_html(&session.document).map_err(|err| err.to_string())
}

#[tauri::command]
fn export_pdf(
    options: Option<PdfExportRequest>,
    state: State<'_, AppState>,
) -> Result<Vec<u8>, String> {
    let session = lock_session(&state)?;
    let options = pdf_export_options_from_request(options)?;
    word_export::export_pdf_with_options(&session.document, options).map_err(safe_pdf_export_error)
}

#[tauri::command]
fn export_txt_to_path(
    path: String,
    state: State<'_, AppState>,
) -> Result<ExportFileResult, String> {
    let path = validate_path(&path, "txt")?;
    let session = lock_session(&state)?;
    let text = word_export::export_txt(&session.document).map_err(|err| err.to_string())?;
    write_export_bytes_to_path("txt", &path, text.as_bytes())
}

#[tauri::command]
fn export_html_to_path(
    path: String,
    state: State<'_, AppState>,
) -> Result<ExportFileResult, String> {
    let path = validate_path(&path, "html")?;
    let session = lock_session(&state)?;
    let html = word_export::export_html(&session.document).map_err(|err| err.to_string())?;
    write_export_bytes_to_path("html", &path, html.as_bytes())
}

#[tauri::command]
fn export_pdf_to_path(
    path: String,
    options: Option<PdfExportRequest>,
    state: State<'_, AppState>,
) -> Result<ExportFileResult, String> {
    let path = validate_path(&path, "pdf")?;
    let session = lock_session(&state)?;
    let options = pdf_export_options_from_request(options)?;
    let pdf = word_export::export_pdf_with_options(&session.document, options)
        .map_err(safe_pdf_export_error)?;
    write_export_bytes_to_path("pdf", &path, &pdf)
}

#[tauri::command]
fn export_docx_to_path(
    path: String,
    state: State<'_, AppState>,
) -> Result<ExportFileResult, String> {
    let path = validate_path(&path, "docx")?;
    let session = lock_session(&state)?;
    let docx = word_docx::write_docx_bytes(&session.document).map_err(safe_docx_export_error)?;
    write_export_bytes_to_path("docx", &path, &docx)
}

#[tauri::command]
fn prepare_print_html(state: State<'_, AppState>) -> Result<String, String> {
    let session = lock_session(&state)?;
    word_export::export_print_html(&session.document).map_err(|err| err.to_string())
}

#[tauri::command]
fn check_spelling(
    text: String,
    language_tag: String,
    app: tauri::AppHandle,
) -> Result<SpellCheckResult, String> {
    let user_root = user_dictionary_dir(&app)?;
    ensure_user_dictionary_dir(&user_root)?;
    check_spelling_with_root(&text, &language_tag, &user_root)
}

#[tauri::command]
fn add_to_personal_dictionary(
    word: String,
    language_tag: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let user_root = user_dictionary_dir(&app)?;
    ensure_user_dictionary_dir(&user_root)?;
    word_spell::add_personal_word(&user_root, &language_tag, &word).map_err(|err| err.to_string())
}

#[tauri::command]
fn list_personal_dictionary_words(
    language_tag: String,
    app: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    let user_root = user_dictionary_dir(&app)?;
    ensure_user_dictionary_dir(&user_root)?;
    list_personal_dictionary_words_with_root(&language_tag, &user_root)
}

#[tauri::command]
fn remove_from_personal_dictionary(
    word: String,
    language_tag: String,
    app: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    let user_root = user_dictionary_dir(&app)?;
    ensure_user_dictionary_dir(&user_root)?;
    remove_personal_dictionary_word_with_root(&word, &language_tag, &user_root)
}

#[tauri::command]
fn list_dictionaries(app: tauri::AppHandle) -> Result<Vec<DictionaryInfo>, String> {
    let user_root = user_dictionary_dir(&app)?;
    ensure_user_dictionary_dir(&user_root)?;
    Ok(word_spell::list_dictionaries_with_user_root(&user_root))
}

#[tauri::command]
fn install_user_dictionary(
    language_tag: String,
    aff_path: String,
    dic_path: String,
    app: tauri::AppHandle,
) -> Result<DictionaryInfo, String> {
    let user_root = user_dictionary_dir(&app)?;
    ensure_user_dictionary_dir(&user_root)?;
    install_user_dictionary_with_root(&language_tag, &aff_path, &dic_path, &user_root)
}

#[tauri::command]
fn remove_user_dictionary(language_tag: String, app: tauri::AppHandle) -> Result<(), String> {
    let user_root = user_dictionary_dir(&app)?;
    ensure_user_dictionary_dir(&user_root)?;
    remove_user_dictionary_with_root(&language_tag, &user_root)
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Settings {
    let Ok(path) = settings_path(&app) else {
        return Settings::default();
    };
    load_settings_from_path(&path)
}

#[tauri::command]
fn update_settings(settings: Settings, app: tauri::AppHandle) -> Result<Settings, String> {
    let path = settings_path(&app)?;
    save_settings_to_path(&path, settings)
}

#[tauri::command]
fn reset_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    let path = settings_path(&app)?;
    reset_settings_at_path(&path)
}

fn sanitize_settings(settings: Settings) -> Settings {
    Settings {
        telemetry_enabled: false,
        language_tag: normalize_language_setting(&settings.language_tag),
        ui_locale: normalize_ui_locale(&settings.ui_locale),
        high_contrast: settings.high_contrast,
        large_toolbar: settings.large_toolbar,
        reduced_motion: settings.reduced_motion,
        low_resource: settings.low_resource,
        smart_typing: settings.smart_typing,
    }
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join(SETTINGS_FILE_NAME))
        .map_err(|_| "settings storage is unavailable".to_string())
}

fn load_settings_from_path(path: &Path) -> Settings {
    try_load_settings_from_path(path).unwrap_or_default()
}

fn try_load_settings_from_path(path: &Path) -> Result<Settings, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Settings::default());
        }
        Err(error) => return Err(safe_settings_io_error(error)),
    };
    let file_type = metadata.file_type();
    if file_type.is_symlink() || !file_type.is_file() || metadata.len() > MAX_SETTINGS_BYTES {
        return Ok(Settings::default());
    }

    let bytes = fs::read(path).map_err(safe_settings_io_error)?;
    serde_json::from_slice::<Settings>(&bytes)
        .map(sanitize_settings)
        .map_err(|_| "settings could not be read".to_string())
}

fn save_settings_to_path(path: &Path, settings: Settings) -> Result<Settings, String> {
    let settings = sanitize_settings(settings);
    ensure_private_settings_parent(path)?;
    let bytes = serde_json::to_vec_pretty(&settings)
        .map_err(|_| "settings could not be saved".to_string())?;
    write_bytes_atomically(path, &bytes, true)
        .map_err(|_| "settings could not be saved".to_string())?;
    Ok(settings)
}

fn reset_settings_at_path(path: &Path) -> Result<Settings, String> {
    save_settings_to_path(path, Settings::default())
}

fn ensure_private_settings_parent(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .ok_or_else(|| "settings storage is unavailable".to_string())?;
    fs::create_dir_all(parent).map_err(safe_settings_io_error)?;
    let metadata = fs::symlink_metadata(parent).map_err(safe_settings_io_error)?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err("settings storage is unavailable".to_string());
    }
    set_private_directory_permissions(parent)
        .map_err(|_| "settings storage is unavailable".to_string())
}

fn safe_settings_io_error(error: std::io::Error) -> String {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => "permission denied".to_string(),
        _ => "settings storage is unavailable".to_string(),
    }
}

fn check_spelling_with_root(
    text: &str,
    language_tag: &str,
    user_root: &Path,
) -> Result<SpellCheckResult, String> {
    let mut warnings = Vec::new();
    let checker = match word_spell::checker_for_with_user_root(language_tag, user_root) {
        Ok(checker) => checker,
        Err(word_spell::SpellError::MissingDictionary { .. }) => {
            warnings.push("Selected dictionary is unavailable; checked with the bundled English bootstrap dictionary.".to_string());
            word_spell::checker_for(FALLBACK_LANGUAGE_TAG).map_err(|err| err.to_string())?
        }
        Err(err) => return Err(err.to_string()),
    };
    let personal_words = word_spell::read_personal_words(user_root, checker.language_tag())
        .map_err(|err| err.to_string())?;
    Ok(SpellCheckResult {
        language_tag: checker.language_tag().to_string(),
        dictionary_display_name: checker.display_name().to_string(),
        issues: checker.check_with_personal_words(text, &personal_words),
        warnings,
    })
}

fn list_personal_dictionary_words_with_root(
    language_tag: &str,
    user_root: &Path,
) -> Result<Vec<String>, String> {
    word_spell::list_personal_words(user_root, language_tag).map_err(safe_personal_dictionary_error)
}

fn remove_personal_dictionary_word_with_root(
    word: &str,
    language_tag: &str,
    user_root: &Path,
) -> Result<Vec<String>, String> {
    word_spell::remove_personal_word(user_root, language_tag, word)
        .map_err(safe_personal_dictionary_error)
}

fn install_user_dictionary_with_root(
    language_tag: &str,
    aff_path: &str,
    dic_path: &str,
    user_root: &Path,
) -> Result<DictionaryInfo, String> {
    let aff_path = validate_dictionary_install_path(aff_path, "aff")?;
    let dic_path = validate_dictionary_install_path(dic_path, "dic")?;
    word_spell::install_user_dictionary(user_root, language_tag, &aff_path, &dic_path)
        .map_err(safe_dictionary_install_error)
}

fn remove_user_dictionary_with_root(language_tag: &str, user_root: &Path) -> Result<(), String> {
    word_spell::remove_user_dictionary(user_root, language_tag)
        .map_err(safe_dictionary_removal_error)
}

fn safe_personal_dictionary_error(error: word_spell::SpellError) -> String {
    match error {
        word_spell::SpellError::InvalidDictionary {
            reason: "personal dictionary word is invalid",
        } => "personal dictionary word is invalid".to_string(),
        _ => "personal dictionary is unavailable".to_string(),
    }
}

fn safe_dictionary_install_error(error: word_spell::SpellError) -> String {
    match error {
        word_spell::SpellError::InvalidDictionary {
            reason: "language tag is invalid",
        } => "invalid language".to_string(),
        word_spell::SpellError::InvalidDictionary {
            reason: "dictionary source file is unsupported",
        }
        | word_spell::SpellError::InvalidDictionary {
            reason: "dictionary file is too large",
        }
        | word_spell::SpellError::InvalidDictionary {
            reason: "only UTF-8 Hunspell dictionaries are supported",
        }
        | word_spell::SpellError::InvalidDictionary {
            reason: "dictionary word list is empty",
        } => "unsupported file".to_string(),
        _ => "dictionary could not be installed".to_string(),
    }
}

fn safe_dictionary_removal_error(error: word_spell::SpellError) -> String {
    match error {
        word_spell::SpellError::InvalidDictionary {
            reason: "language tag is invalid",
        } => "invalid language".to_string(),
        _ => "dictionary could not be removed".to_string(),
    }
}

fn user_dictionary_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join(USER_DICTIONARY_DIR_NAME))
        .map_err(|_| "app data directory is unavailable".to_string())
}

fn ensure_user_dictionary_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(safe_io_error)?;
    let metadata = fs::symlink_metadata(path).map_err(safe_io_error)?;
    if !metadata.file_type().is_dir() {
        return Err("dictionary directory is unavailable".to_string());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(safe_io_error)?;
    }
    Ok(())
}

fn validate_dictionary_install_path(
    path: &str,
    expected_extension: &str,
) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err("unsupported file".to_string());
    }

    let path = PathBuf::from(path);
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err("unsupported file".to_string());
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension != expected_extension {
        return Err("unsupported file".to_string());
    }

    Ok(path)
}

fn normalize_language_setting(language_tag: &str) -> String {
    let normalized = language_tag.trim().replace('_', "-");
    if is_safe_language_tag(&normalized) {
        normalized
    } else {
        FALLBACK_LANGUAGE_TAG.to_string()
    }
}

fn is_safe_language_tag(language_tag: &str) -> bool {
    if language_tag.is_empty() || language_tag.len() > 35 {
        return false;
    }
    let mut segments = language_tag.split('-');
    let Some(primary) = segments.next() else {
        return false;
    };
    if !(2..=3).contains(&primary.len()) || !primary.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return false;
    }
    segments.all(|segment| {
        !segment.is_empty()
            && segment.len() <= 8
            && segment.chars().all(|ch| ch.is_ascii_alphanumeric())
    })
}

fn normalize_ui_locale(ui_locale: &str) -> String {
    let ui_locale = ui_locale.trim();
    match ui_locale {
        "en-US" | "es-ES" | "ar" => ui_locale.to_string(),
        _ => "en-US".to_string(),
    }
}

fn template_summaries() -> Vec<TemplateSummary> {
    vec![
        TemplateSummary {
            id: "blank".to_string(),
            name: "Blank".to_string(),
            description: "Untitled document with one body paragraph".to_string(),
        },
        TemplateSummary {
            id: "report".to_string(),
            name: "School Report".to_string(),
            description: "Class report outline with sections and bullet prompts".to_string(),
        },
        TemplateSummary {
            id: "letter".to_string(),
            name: "Formal Letter".to_string(),
            description: "Formal correspondence with generic placeholders".to_string(),
        },
        TemplateSummary {
            id: "project-report".to_string(),
            name: "Project Report".to_string(),
            description: "NGO or project update with milestone table".to_string(),
        },
        TemplateSummary {
            id: "resume".to_string(),
            name: "CV / Resume".to_string(),
            description: "Simple resume structure with placeholder sections".to_string(),
        },
        TemplateSummary {
            id: "meeting-minutes".to_string(),
            name: "Meeting Minutes".to_string(),
            description: "Agenda, decisions, and action-item tracker".to_string(),
        },
        TemplateSummary {
            id: "memo".to_string(),
            name: "Memo".to_string(),
            description: "Short internal memo with purpose and next steps".to_string(),
        },
        TemplateSummary {
            id: "invoice".to_string(),
            name: "Invoice".to_string(),
            description: "Invoice-style table layout with generic line items".to_string(),
        },
        TemplateSummary {
            id: "flyer".to_string(),
            name: "Flyer One-Pager".to_string(),
            description: "One-page announcement starter without embedded assets".to_string(),
        },
    ]
}

fn build_template_document(template_id: &str) -> Result<Document, String> {
    match template_id {
        "blank" => Ok(Document::new_untitled()),
        "report" => Ok(template_document(
            "Untitled School Report",
            vec![
                heading_block(1, "School Report"),
                paragraph_block("Topic"),
                paragraph_block("Student name"),
                paragraph_block("Course or class"),
                heading_block(2, "Introduction"),
                paragraph_block("Add a short introduction here."),
                heading_block(2, "Key Points"),
                unordered_list_block(&[
                    "Add the first key point here.",
                    "Add supporting evidence here.",
                    "Add a final observation here.",
                ]),
                heading_block(2, "Conclusion"),
                paragraph_block("Add a short conclusion here."),
            ],
        )),
        "letter" => Ok(template_document(
            "Untitled Formal Letter",
            vec![
                paragraph_block("Date"),
                paragraph_block("Recipient name"),
                paragraph_block("Recipient role"),
                paragraph_block("Subject: Purpose of the letter"),
                paragraph_block("Dear recipient,"),
                paragraph_block("Add the opening paragraph here."),
                paragraph_block("Add supporting details here."),
                paragraph_block("Add the closing paragraph here."),
                paragraph_block("Sincerely,"),
                paragraph_block("Sender name"),
            ],
        )),
        "project-report" => Ok(template_document(
            "Untitled Project Report",
            vec![
                heading_block(1, "Project Report"),
                paragraph_block("Project title"),
                paragraph_block("Reporting period"),
                heading_block(2, "Overview"),
                paragraph_block("Summarize the project purpose and current status here."),
                heading_block(2, "Milestones"),
                table_block(vec![
                    vec!["Milestone", "Status", "Notes"],
                    vec!["Milestone one", "Not started", "Add notes."],
                    vec!["Milestone two", "In progress", "Add notes."],
                    vec!["Milestone three", "Planned", "Add notes."],
                ]),
                heading_block(2, "Risks"),
                unordered_list_block(&[
                    "Add an important risk here.",
                    "Add a mitigation step here.",
                ]),
                heading_block(2, "Next Steps"),
                paragraph_block("Add next steps here."),
            ],
        )),
        "resume" => Ok(template_document(
            "Untitled Resume",
            vec![
                heading_block(1, "Resume"),
                paragraph_block("Candidate name"),
                paragraph_block("Email | Phone | Location"),
                heading_block(2, "Summary"),
                paragraph_block("Add a short professional summary here."),
                heading_block(2, "Skills"),
                unordered_list_block(&[
                    "Add a skill here.",
                    "Add another skill here.",
                    "Add a tool or method here.",
                ]),
                heading_block(2, "Experience"),
                paragraph_block("Role title - Organization or project"),
                unordered_list_block(&[
                    "Add a responsibility or result here.",
                    "Add another responsibility or result here.",
                ]),
                heading_block(2, "Education"),
                paragraph_block("Program or credential - Institution"),
            ],
        )),
        "meeting-minutes" => Ok(template_document(
            "Untitled Meeting Minutes",
            vec![
                heading_block(1, "Meeting Minutes"),
                paragraph_block("Meeting topic"),
                paragraph_block("Date and time"),
                paragraph_block("Attendees"),
                heading_block(2, "Agenda"),
                ordered_list_block(&[
                    "Add agenda item one.",
                    "Add agenda item two.",
                    "Add agenda item three.",
                ]),
                heading_block(2, "Decisions"),
                unordered_list_block(&["Add a decision here.", "Add another decision here."]),
                heading_block(2, "Action Items"),
                table_block(vec![
                    vec!["Action", "Owner", "Due date", "Status"],
                    vec!["Action item", "Owner", "YYYY-MM-DD", "Open"],
                    vec!["Action item", "Owner", "YYYY-MM-DD", "Open"],
                ]),
            ],
        )),
        "memo" => Ok(template_document(
            "Untitled Memo",
            vec![
                heading_block(1, "Memo"),
                paragraph_block("To: Audience"),
                paragraph_block("From: Sender"),
                paragraph_block("Date: YYYY-MM-DD"),
                paragraph_block("Subject: Topic"),
                heading_block(2, "Purpose"),
                paragraph_block("Add the purpose of this memo here."),
                heading_block(2, "Background"),
                paragraph_block("Add relevant context here."),
                heading_block(2, "Requested Action"),
                paragraph_block("Add the requested action or decision here."),
            ],
        )),
        "invoice" => Ok(template_document(
            "Untitled Invoice",
            vec![
                heading_block(1, "Invoice"),
                table_block(vec![
                    vec!["Field", "Value"],
                    vec!["Invoice number", "INV-0000"],
                    vec!["Invoice date", "YYYY-MM-DD"],
                    vec!["Due date", "YYYY-MM-DD"],
                ]),
                heading_block(2, "Bill To"),
                paragraph_block("Recipient or billing contact"),
                heading_block(2, "Line Items"),
                table_block(vec![
                    vec!["Description", "Quantity", "Unit price", "Amount"],
                    vec!["Service or item", "0", "0.00", "0.00"],
                    vec!["Service or item", "0", "0.00", "0.00"],
                    vec!["Subtotal", "", "", "0.00"],
                    vec!["Tax", "", "", "0.00"],
                    vec!["Total", "", "", "0.00"],
                ]),
                paragraph_block("Payment notes"),
            ],
        )),
        "flyer" => Ok(template_document(
            "Untitled Flyer",
            vec![
                heading_block(1, "Event Or Announcement"),
                paragraph_block("Short headline"),
                heading_block(2, "When"),
                paragraph_block("Date and time"),
                heading_block(2, "Where"),
                paragraph_block("Location"),
                heading_block(2, "Details"),
                paragraph_block("Add a brief invitation or announcement here."),
                heading_block(2, "Contact"),
                paragraph_block("Contact method"),
            ],
        )),
        _ => Err("template is unavailable".to_string()),
    }
}

fn template_document(title: &str, blocks: Vec<Block>) -> Document {
    let mut document = Document::new_untitled();
    document.meta.title = title.to_string();
    document.sections[0].blocks = blocks;
    document
}

fn paragraph_block(text: &str) -> Block {
    Block::Paragraph(Paragraph {
        bookmark_id: None,
        style: StyleId::from("body"),
        format: Default::default(),
        inlines: vec![Inline::text(text)],
    })
}

fn heading_block(level: u8, text: &str) -> Block {
    Block::Heading(Heading {
        bookmark_id: None,
        level,
        inlines: vec![Inline::text(text)],
    })
}

fn unordered_list_block(items: &[&str]) -> Block {
    list_block("900w-unordered", items)
}

fn ordered_list_block(items: &[&str]) -> Block {
    list_block("900w-ordered", items)
}

fn list_block(definition_id: &str, items: &[&str]) -> Block {
    Block::List(ListBlock {
        definition_id: definition_id.to_string(),
        items: items
            .iter()
            .map(|item| ListItem {
                level: 1,
                blocks: vec![paragraph_block(item)],
            })
            .collect(),
    })
}

fn table_block(rows: Vec<Vec<&str>>) -> Block {
    Block::Table(Table {
        rows: rows
            .into_iter()
            .map(|cells| TableRow {
                cells: cells
                    .into_iter()
                    .map(|cell| TableCell {
                        presentation: Default::default(),
                        blocks: vec![paragraph_block(cell)],
                    })
                    .collect(),
            })
            .collect(),
    })
}

fn import_image_into_session(
    session: &mut DocumentSession,
    path: &Path,
    section_index: Option<usize>,
    block_index: Option<usize>,
) -> Result<Document, String> {
    let (extension, media_type, bytes) = read_validated_image(path)?;
    let asset_id = unique_image_asset_id(&session.document, extension);
    let block = Block::Image(ImageBlock {
        asset_id: asset_id.clone(),
        presentation: ImagePresentation::default(),
        alt_text: Some("Image".to_string()),
    });

    let mut document = session.document.clone();
    session
        .undo
        .apply_mutation(&mut document, move |document| {
            let target_section = section_index.unwrap_or(0);
            let section_len = document
                .sections
                .get(target_section)
                .map(|section| section.blocks.len())
                .or_else(|| {
                    document
                        .sections
                        .first()
                        .map(|section| section.blocks.len())
                })
                .ok_or(DocumentError::SectionOutOfBounds { section_index: 0 })?;
            let target_section = if document.sections.get(target_section).is_some() {
                target_section
            } else {
                0
            };
            let target_block = block_index.unwrap_or(section_len).min(section_len);

            document.assets.insert(
                asset_id.clone(),
                AssetRef {
                    id: asset_id,
                    media_type: media_type.to_string(),
                    byte_len: bytes.len(),
                    bytes,
                    original_name: None,
                },
            );
            document.apply_command(DocumentCommand::InsertBlock {
                section_index: target_section,
                block_index: target_block,
                block,
            })
        })
        .map_err(|_| "image could not be inserted".to_string())?;
    session.document = document;
    session.dirty = true;
    Ok(session.document.clone())
}

fn unique_image_asset_id(document: &Document, extension: &'static str) -> String {
    loop {
        let id = format!("image-{}.{}", uuid::Uuid::new_v4(), extension);
        if !document.assets.contains_key(&id) {
            return id;
        }
    }
}

fn validate_image_path(path: &str) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err("image file is unsupported".to_string());
    }

    let path = PathBuf::from(path);
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err("image file is unsupported".to_string());
    }

    let extension = image_extension(&path)?;
    if image_media_type_for_extension(extension).is_none() {
        return Err("image file is unsupported".to_string());
    }

    Ok(path)
}

fn read_validated_image(path: &Path) -> Result<(&'static str, &'static str, Vec<u8>), String> {
    let extension = image_extension(path)?;
    let expected_media_type = image_media_type_for_extension(extension)
        .ok_or_else(|| "image file is unsupported".to_string())?;
    let metadata = fs::symlink_metadata(path).map_err(safe_io_error)?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err("image file is unsupported".to_string());
    }
    if metadata.len() == 0 {
        return Err("image file is unsupported".to_string());
    }
    if metadata.len() > MAX_IMAGE_BYTES {
        return Err(IMAGE_TOO_LARGE_ERROR.to_string());
    }

    let mut bytes = fs::read(path).map_err(safe_io_error)?;
    if bytes.is_empty() {
        return Err("image file is unsupported".to_string());
    }
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        return Err(IMAGE_TOO_LARGE_ERROR.to_string());
    }
    let detected_media_type =
        detect_image_media_type(&bytes).ok_or_else(|| "image file is unsupported".to_string())?;
    if detected_media_type != expected_media_type {
        return Err("image file is unsupported".to_string());
    }
    if detected_media_type == "image/jpeg" {
        bytes = strip_jpeg_import_metadata(&bytes)
            .ok_or_else(|| "image file is unsupported".to_string())?;
        if bytes.len() as u64 > MAX_IMAGE_BYTES {
            return Err(IMAGE_TOO_LARGE_ERROR.to_string());
        }
    }

    Ok((extension, detected_media_type, bytes))
}

fn strip_jpeg_import_metadata(bytes: &[u8]) -> Option<Vec<u8>> {
    if !bytes.starts_with(b"\xff\xd8") {
        return None;
    }

    let mut sanitized = vec![0xff, 0xd8];
    let mut index = 2;
    let mut saw_scan = false;
    loop {
        if index >= bytes.len() || bytes[index] != 0xff {
            return None;
        }
        while index < bytes.len() && bytes[index] == 0xff {
            index += 1;
        }
        if index >= bytes.len() {
            return None;
        }
        let marker = bytes[index];
        index += 1;

        if marker == 0x00 || marker == 0xd8 {
            return None;
        }
        if marker == 0xd9 {
            sanitized.extend_from_slice(&[0xff, marker]);
            return (saw_scan && index == bytes.len()).then_some(sanitized);
        }
        if is_jpeg_import_metadata_marker(marker) {
            if saw_scan {
                return None;
            }
            index = jpeg_segment_end(bytes, index)?;
            continue;
        }
        if is_jpeg_import_marker_without_payload(marker) {
            if !saw_scan && (0xd0..=0xd7).contains(&marker) {
                return None;
            }
            sanitized.extend_from_slice(&[0xff, marker]);
            continue;
        }

        let segment_end = jpeg_segment_end(bytes, index)?;
        sanitized.extend_from_slice(&[0xff, marker]);
        sanitized.extend_from_slice(&bytes[index..segment_end]);
        index = segment_end;

        if marker == 0xda {
            saw_scan = true;
            index = copy_jpeg_scan_data_until_marker(bytes, index, &mut sanitized)?;
        }
    }
}

fn jpeg_segment_end(bytes: &[u8], length_index: usize) -> Option<usize> {
    if length_index + 2 > bytes.len() {
        return None;
    }
    let segment_length =
        u16::from_be_bytes([bytes[length_index], bytes[length_index + 1]]) as usize;
    if segment_length < 2 {
        return None;
    }
    let segment_end = length_index.checked_add(segment_length)?;
    if segment_end > bytes.len() {
        return None;
    }
    Some(segment_end)
}

fn copy_jpeg_scan_data_until_marker(
    bytes: &[u8],
    scan_start: usize,
    sanitized: &mut Vec<u8>,
) -> Option<usize> {
    let mut index = scan_start;
    while index < bytes.len() {
        if bytes[index] != 0xff {
            index += 1;
            continue;
        }
        let marker_start = index;
        let mut marker_index = index + 1;
        while marker_index < bytes.len() && bytes[marker_index] == 0xff {
            marker_index += 1;
        }
        if marker_index >= bytes.len() {
            return None;
        }
        let marker = bytes[marker_index];
        if marker == 0x00 || (0xd0..=0xd7).contains(&marker) {
            index = marker_index + 1;
            continue;
        }
        sanitized.extend_from_slice(&bytes[scan_start..marker_start]);
        return Some(marker_start);
    }
    None
}

fn is_jpeg_import_marker_without_payload(marker: u8) -> bool {
    marker == 0x01 || (0xd0..=0xd9).contains(&marker)
}

fn is_jpeg_import_metadata_marker(marker: u8) -> bool {
    (0xe0..=0xef).contains(&marker) || marker == 0xfe
}

fn image_extension(path: &Path) -> Result<&'static str, String> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => Ok("png"),
        "jpg" | "jpeg" => Ok("jpg"),
        "gif" => Ok("gif"),
        "webp" => Ok("webp"),
        _ => Err("image file is unsupported".to_string()),
    }
}

fn image_media_type_for_extension(extension: &str) -> Option<&'static str> {
    match extension {
        "png" => Some("image/png"),
        "jpg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
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

fn write_document_to_path(document: &Document, path: &Path) -> Result<(), String> {
    let bytes = word_odf::write_odt_bytes(document).map_err(|err| err.to_string())?;
    write_bytes_atomically(path, &bytes, false)
}

fn read_document_from_path(path: &Path) -> Result<Document, String> {
    let metadata = std::fs::metadata(path).map_err(safe_io_error)?;
    if metadata.len() > MAX_DOCUMENT_BYTES {
        return Err("document exceeds supported bootstrap size limit".to_string());
    }
    let bytes = std::fs::read(path).map_err(safe_io_error)?;
    word_odf::read_odt_bytes(&bytes).map_err(|err| err.to_string())
}

fn read_docx_document_from_path(path: &Path) -> Result<Document, String> {
    let metadata = std::fs::metadata(path).map_err(safe_io_error)?;
    if metadata.len() > MAX_DOCUMENT_BYTES {
        return Err("document exceeds supported bootstrap size limit".to_string());
    }
    let bytes = std::fs::read(path).map_err(safe_io_error)?;
    word_docx::read_docx_bytes(&bytes).map_err(safe_docx_import_error)
}

fn safe_docx_import_error(error: word_docx::DocxError) -> String {
    match error {
        word_docx::DocxError::PackageTooLarge => {
            "DOCX package exceeds supported size limit".to_string()
        }
        word_docx::DocxError::TooManyEntries { .. } => {
            "DOCX package contains too many entries".to_string()
        }
        word_docx::DocxError::EntryTooLarge { .. } => {
            "DOCX package entry exceeds supported size limit".to_string()
        }
        word_docx::DocxError::ExpandedSizeTooLarge => {
            "DOCX package expands beyond supported size limit".to_string()
        }
        word_docx::DocxError::UnsafePath { .. }
        | word_docx::DocxError::PathTooDeep { .. }
        | word_docx::DocxError::SymlinkEntry { .. }
        | word_docx::DocxError::EncryptedEntry { .. }
        | word_docx::DocxError::ExecutableEntry { .. } => {
            "DOCX package contains unsupported or unsafe entries".to_string()
        }
        word_docx::DocxError::MissingDocument => {
            "DOCX package is missing document content".to_string()
        }
        word_docx::DocxError::Xml { .. }
        | word_docx::DocxError::XmlTooDeep { .. }
        | word_docx::DocxError::XmlEntityDeclaration { .. } => {
            "DOCX package contains unsupported XML".to_string()
        }
        word_docx::DocxError::Zip(_) | word_docx::DocxError::Io(_) => {
            "DOCX package could not be read".to_string()
        }
    }
}

fn safe_docx_export_error(_error: word_docx::DocxError) -> String {
    "DOCX export could not be prepared".to_string()
}

fn pdf_export_options_from_request(
    request: Option<PdfExportRequest>,
) -> Result<PdfExportOptions, String> {
    let Some(request) = request else {
        return Ok(PdfExportOptions::default());
    };
    let page_range = match (request.page_start, request.page_end) {
        (None, None) => None,
        (Some(start), Some(end)) if start > 0 && end >= start => Some(PdfPageRange { start, end }),
        _ => return Err("PDF page range is invalid".to_string()),
    };
    Ok(PdfExportOptions { page_range })
}

fn safe_pdf_export_error(error: word_export::ExportError) -> String {
    match error {
        word_export::ExportError::EmptyDocument => "document has no sections".to_string(),
        word_export::ExportError::InvalidPdfPageRange => "PDF page range is invalid".to_string(),
    }
}

fn write_export_bytes_to_path(
    format: &str,
    path: &Path,
    bytes: &[u8],
) -> Result<ExportFileResult, String> {
    write_bytes_atomically(path, bytes, false)?;
    Ok(ExportFileResult {
        format: format.to_string(),
        byte_len: bytes.len() as u64,
    })
}

fn remember_recent_path(session: &mut DocumentSession, path: PathBuf) {
    if let Some(index) = session
        .recent_documents
        .iter()
        .position(|entry| entry.path == path)
    {
        let entry = session.recent_documents.remove(index);
        session.recent_documents.insert(0, entry);
        return;
    }

    let token = format!("recent-{}", session.next_recent_id);
    session.next_recent_id += 1;
    let label = format!("Recent document {}", session.next_recent_id - 1);
    session
        .recent_documents
        .insert(0, RecentEntry { token, path, label });
    session.recent_documents.truncate(MAX_RECENT_DOCUMENTS);
}

fn document_file_state_from_session(
    session: &DocumentSession,
) -> Result<DocumentFileState, String> {
    Ok(DocumentFileState {
        has_current_path: session.current_path.is_some(),
        dirty: session.dirty,
        recent_documents: recent_summaries(session),
        recovery_documents: list_recovery_documents_from_disk()?,
    })
}

fn recent_summaries(session: &DocumentSession) -> Vec<RecentDocumentSummary> {
    session
        .recent_documents
        .iter()
        .map(|entry| RecentDocumentSummary {
            token: entry.token.clone(),
            label: entry.label.clone(),
            is_current: session.current_path.as_ref() == Some(&entry.path),
        })
        .collect()
}

fn write_recovery_document(document: &Document) -> Result<RecoveryDocumentSummary, String> {
    write_recovery_document_in_dir(document, &recovery_dir())
}

fn write_recovery_document_in_dir(
    document: &Document,
    dir: &Path,
) -> Result<RecoveryDocumentSummary, String> {
    let bytes = word_odf::write_odt_bytes(document).map_err(|err| err.to_string())?;
    ensure_private_recovery_dir_at(dir)?;

    for _ in 0..3 {
        let token = recovery_token_for_document(document);
        let path = recovery_path_for_token_in_dir(&token, dir)?;
        if path.exists() {
            continue;
        }
        write_bytes_atomically(&path, &bytes, true)?;
        prune_recovery_snapshots_in_dir(dir, Some(&token))?;
        let summaries = list_recovery_documents_in_dir(dir)?;
        if let Some(summary) = summaries.into_iter().find(|summary| summary.token == token) {
            return Ok(summary);
        }
        return recovery_summary_from_path(token, &path, 1);
    }

    Err("recovery snapshot could not be created".to_string())
}

fn list_recovery_documents_from_disk() -> Result<Vec<RecoveryDocumentSummary>, String> {
    list_recovery_documents_in_dir(&recovery_dir())
}

fn list_recovery_documents_in_dir(dir: &Path) -> Result<Vec<RecoveryDocumentSummary>, String> {
    let mut entries = recovery_snapshot_entries_in_dir(dir)?;
    sort_recovery_entries_newest_first(&mut entries);

    entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| recovery_summary_from_entry(entry, index + 1))
        .collect()
}

fn recovery_snapshot_entries_in_dir(dir: &Path) -> Result<Vec<RecoverySnapshotEntry>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(safe_io_error)? {
        let entry = entry.map_err(safe_io_error)?;
        let Some(token) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };
        let Ok(parts) = parse_recovery_token(&token) else {
            continue;
        };
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path).map_err(safe_io_error)?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() || !file_type.is_file() {
            continue;
        }
        let modified_duration = metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok());
        let modified_unix_seconds = modified_duration
            .as_ref()
            .map(|duration| duration.as_secs())
            .unwrap_or_else(current_unix_seconds);
        let modified_sort_key = modified_duration
            .map(|duration| duration.as_nanos())
            .unwrap_or_else(current_unix_nanos);
        entries.push(RecoverySnapshotEntry {
            token,
            document_key: parts.document_key,
            path,
            modified_sort_key,
            modified_unix_seconds,
            byte_len: metadata.len(),
        });
    }

    Ok(entries)
}

fn prune_recovery_snapshots_in_dir(dir: &Path, pinned_token: Option<&str>) -> Result<(), String> {
    let mut entries = recovery_snapshot_entries_in_dir(dir)?;
    sort_recovery_entries_newest_first(&mut entries);
    if let Some(pinned_token) = pinned_token {
        if let Some(index) = entries
            .iter()
            .position(|entry| entry.token.as_str() == pinned_token)
        {
            let entry = entries.remove(index);
            entries.insert(0, entry);
        }
    }

    let mut kept_tokens = HashSet::new();
    let mut kept_per_document = BTreeMap::new();
    for entry in &entries {
        let document_count = kept_per_document
            .entry(entry.document_key.clone())
            .or_insert(0);
        if *document_count >= RECOVERY_SNAPSHOTS_PER_DOCUMENT {
            continue;
        }
        if kept_tokens.len() >= MAX_RECOVERY_SNAPSHOTS {
            continue;
        }
        kept_tokens.insert(entry.token.clone());
        *document_count += 1;
    }

    for entry in entries {
        if kept_tokens.contains(&entry.token) {
            continue;
        }
        match std::fs::remove_file(entry.path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(safe_io_error(error)),
        }
    }
    Ok(())
}

fn sort_recovery_entries_newest_first(entries: &mut [RecoverySnapshotEntry]) {
    entries.sort_by(|left, right| {
        right
            .modified_sort_key
            .cmp(&left.modified_sort_key)
            .then_with(|| right.token.cmp(&left.token))
    });
}

fn recovery_summary_from_entry(
    entry: RecoverySnapshotEntry,
    index: usize,
) -> Result<RecoveryDocumentSummary, String> {
    Ok(RecoveryDocumentSummary {
        token: entry.token,
        label: format!("Recovery draft {index}"),
        modified_unix_seconds: entry.modified_unix_seconds,
        byte_len: entry.byte_len,
    })
}

fn recovery_summary_from_path(
    token: String,
    path: &Path,
    index: usize,
) -> Result<RecoveryDocumentSummary, String> {
    let metadata = std::fs::metadata(path).map_err(safe_io_error)?;
    let modified_unix_seconds = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or_else(current_unix_seconds);

    Ok(RecoveryDocumentSummary {
        token,
        label: format!("Recovery draft {index}"),
        modified_unix_seconds,
        byte_len: metadata.len(),
    })
}

fn recovery_token_for_document(document: &Document) -> String {
    format!(
        "recovery-v1-{}-{}.odt",
        document.id.simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn recovery_path_for_token_in_dir(token: &str, dir: &Path) -> Result<PathBuf, String> {
    let token = validate_recovery_token(token)?;
    Ok(dir.join(token))
}

fn recovery_dir() -> PathBuf {
    std::env::temp_dir().join(RECOVERY_DIR_NAME)
}

fn ensure_private_recovery_dir_at(dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(safe_io_error)?;
    set_private_directory_permissions(dir)?;
    Ok(dir.to_path_buf())
}

fn recover_document_from_dir(
    token: &str,
    dir: &Path,
    session: &mut DocumentSession,
) -> Result<Document, String> {
    let path = recovery_path_for_token_in_dir(token, dir)?;
    validate_recovery_regular_file(&path)?;
    let document = read_document_from_path(&path)?;
    session.document = document;
    session.undo = UndoStack::default();
    session.current_path = None;
    session.dirty = true;
    Ok(session.document.clone())
}

fn validate_recovery_regular_file(path: &Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path).map_err(safe_io_error)?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() || !file_type.is_file() {
        return Err("recovery token is invalid".to_string());
    }
    Ok(())
}

fn write_bytes_atomically(path: &Path, bytes: &[u8], private: bool) -> Result<(), String> {
    if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err("document exceeds supported bootstrap size limit".to_string());
    }

    let parent = path.parent().filter(|value| !value.as_os_str().is_empty());
    let temp_path = atomic_temp_path(path);
    if let Some(parent) = parent {
        if private {
            set_private_directory_permissions(parent)?;
        }
    }

    let write_result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        let mut file = options.open(&temp_path).map_err(safe_io_error)?;
        set_output_file_permissions(&file, path, private)?;
        file.write_all(bytes).map_err(safe_io_error)?;
        file.sync_all().map_err(safe_io_error)?;
        drop(file);
        std::fs::rename(&temp_path, path).map_err(safe_io_error)
    })();

    if write_result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    write_result
}

fn atomic_temp_path(path: &Path) -> PathBuf {
    let temp_name = format!(
        ".900word-write-{}-{}.tmp",
        std::process::id(),
        current_unix_nanos()
    );
    path.with_file_name(temp_name)
}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(safe_io_error)
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(file: &std::fs::File) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    file.set_permissions(std::fs::Permissions::from_mode(0o600))
        .map_err(safe_io_error)
}

#[cfg(not(unix))]
fn set_private_file_permissions(_file: &std::fs::File) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn set_output_file_permissions(
    file: &std::fs::File,
    target_path: &Path,
    private: bool,
) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    if private {
        return set_private_file_permissions(file);
    }

    let permissions = match std::fs::metadata(target_path) {
        Ok(metadata) => {
            let permissions = metadata.permissions();
            if permissions.mode() & 0o222 == 0 {
                return Err("target document is read-only".to_string());
            }
            permissions
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::Permissions::from_mode(0o600)
        }
        Err(error) => return Err(safe_io_error(error)),
    };
    file.set_permissions(permissions).map_err(safe_io_error)
}

#[cfg(not(unix))]
fn set_output_file_permissions(
    file: &std::fs::File,
    target_path: &Path,
    private: bool,
) -> Result<(), String> {
    if private {
        set_private_file_permissions(file)?;
    } else {
        match std::fs::metadata(target_path) {
            Ok(metadata) if metadata.permissions().readonly() => {
                return Err("target document is read-only".to_string());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(safe_io_error(error)),
        }
    }
    Ok(())
}

fn validate_recovery_token(token: &str) -> Result<String, String> {
    parse_recovery_token(token)?;
    Ok(token.to_string())
}

fn parse_recovery_token(token: &str) -> Result<RecoveryTokenParts, String> {
    if token.len() > MAX_RECOVERY_TOKEN_LEN
        || !token.starts_with("recovery-")
        || !token.ends_with(".odt")
        || token.contains("..")
        || token.contains('/')
        || token.contains('\\')
        || !token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '.'))
    {
        return Err("recovery token is invalid".to_string());
    }

    let stem = token
        .strip_suffix(".odt")
        .ok_or_else(|| "recovery token is invalid".to_string())?;
    if let Some(versioned) = stem.strip_prefix("recovery-v1-") {
        let (document_id, snapshot_id) = versioned
            .split_once('-')
            .ok_or_else(|| "recovery token is invalid".to_string())?;
        if snapshot_id.contains('-')
            || parse_simple_uuid_component(document_id).is_err()
            || parse_simple_uuid_component(snapshot_id).is_err()
        {
            return Err("recovery token is invalid".to_string());
        }
        return Ok(RecoveryTokenParts {
            document_key: normalize_uuid_component(document_id)?,
        });
    }

    let legacy_document_id = stem
        .strip_prefix("recovery-")
        .ok_or_else(|| "recovery token is invalid".to_string())?;
    Ok(RecoveryTokenParts {
        document_key: normalize_uuid_component(legacy_document_id)?,
    })
}

fn parse_simple_uuid_component(component: &str) -> Result<(), String> {
    if component.len() != 32 || !component.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err("recovery token is invalid".to_string());
    }
    uuid::Uuid::parse_str(component)
        .map(|_| ())
        .map_err(|_| "recovery token is invalid".to_string())
}

fn normalize_uuid_component(component: &str) -> Result<String, String> {
    uuid::Uuid::parse_str(component)
        .map(|uuid| uuid.simple().to_string())
        .map_err(|_| "recovery token is invalid".to_string())
}

fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn current_unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn lock_session<'a>(
    state: &'a State<'_, AppState>,
) -> Result<std::sync::MutexGuard<'a, DocumentSession>, String> {
    state
        .session
        .lock()
        .map_err(|_| "document session is unavailable".to_string())
}

fn validate_path(path: &str, expected_extension: &str) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err("path must not be empty".to_string());
    }

    let path = PathBuf::from(path);
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err("path contains unsupported traversal".to_string());
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if extension != expected_extension {
        return Err(format!("expected .{expected_extension} document path"));
    }

    Ok(path)
}

fn safe_io_error(error: std::io::Error) -> String {
    match error.kind() {
        std::io::ErrorKind::NotFound => "file not found".to_string(),
        std::io::ErrorKind::PermissionDenied => "permission denied".to_string(),
        _ => "file operation failed".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regular_files_in(root: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(root)
            .expect("test directory should be readable")
            .filter_map(|entry| {
                let path = entry
                    .expect("test directory entry should be readable")
                    .path();
                std::fs::symlink_metadata(&path)
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

    fn write_test_dictionary(root: &Path, stem: &str, words: &[&str]) {
        std::fs::write(
            root.join(format!("{stem}.aff")),
            format!("SET UTF-8\n# test dictionary for {stem}\n"),
        )
        .expect("aff should write");
        let mut dic = format!("{}\n", words.len());
        for word in words {
            dic.push_str(word);
            dic.push('\n');
        }
        std::fs::write(root.join(format!("{stem}.dic")), dic).expect("dic should write");
    }

    #[test]
    fn rejects_wrong_extension() {
        let err = validate_path("document.txt", "odt").expect_err("txt path should fail");

        assert_eq!(err, "expected .odt document path");
    }

    #[test]
    fn pdf_export_request_maps_valid_page_range() {
        let options = pdf_export_options_from_request(Some(PdfExportRequest {
            page_start: Some(2),
            page_end: Some(4),
        }))
        .expect("valid range should map");

        assert_eq!(options.page_range, Some(PdfPageRange { start: 2, end: 4 }));
    }

    #[test]
    fn pdf_export_request_rejects_invalid_page_ranges_generically() {
        let invalid = [
            PdfExportRequest {
                page_start: Some(0),
                page_end: Some(1),
            },
            PdfExportRequest {
                page_start: Some(3),
                page_end: Some(2),
            },
            PdfExportRequest {
                page_start: Some(1),
                page_end: None,
            },
            PdfExportRequest {
                page_start: None,
                page_end: Some(1),
            },
        ];

        for request in invalid {
            let err = pdf_export_options_from_request(Some(request))
                .expect_err("invalid range should fail");
            assert_eq!(err, "PDF page range is invalid");
            assert!(!err.contains('/'));
            assert!(!err.contains("private"));
        }
    }

    #[test]
    fn image_import_embeds_asset_without_path_or_filename() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let image_path = dir.path().join("private-client-logo.png");
        std::fs::write(&image_path, tiny_png()).expect("image should write");
        let mut session = DocumentSession::default();

        let document = import_image_into_session(&mut session, &image_path, Some(0), Some(1))
            .expect("image import should succeed");

        assert!(session.dirty);
        assert_eq!(document.sections[0].blocks.len(), 2);
        let Block::Image(image) = &document.sections[0].blocks[1] else {
            panic!("inserted block should be an image");
        };
        assert!(image.asset_id.starts_with("image-"));
        assert!(image.asset_id.ends_with(".png"));
        let asset = document
            .assets
            .get(&image.asset_id)
            .expect("asset should be embedded");
        assert_eq!(asset.media_type, "image/png");
        assert_eq!(asset.byte_len, tiny_png().len());
        assert_eq!(asset.bytes, tiny_png());
        assert_eq!(asset.original_name, None);

        let serialized = serde_json::to_string(&document).expect("document should serialize");
        assert!(!serialized.contains("private-client-logo"));
        assert!(!serialized.contains(dir.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn image_import_strips_jpeg_metadata_payload_and_updates_byte_len() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let image_path = dir.path().join("private-client-photo.jpg");
        let private_exif = b"PRIVATE-EXIF local/user/photo.jpg";
        let private_comment = b"PRIVATE-COMMENT camera serial";
        let jpeg = jpeg_with_metadata_segment(
            &jpeg_with_metadata_segment(&tiny_jpeg(), 0xe1, private_exif),
            0xfe,
            private_comment,
        );
        std::fs::write(&image_path, jpeg).expect("jpeg should write");
        let mut session = DocumentSession::default();

        let document = import_image_into_session(&mut session, &image_path, Some(0), Some(1))
            .expect("metadata-bearing jpeg should import");

        let Block::Image(image) = &document.sections[0].blocks[1] else {
            panic!("inserted block should be an image");
        };
        assert!(image.asset_id.ends_with(".jpg"));
        let asset = document
            .assets
            .get(&image.asset_id)
            .expect("asset should be embedded");
        assert_eq!(asset.media_type, "image/jpeg");
        assert_eq!(asset.byte_len, tiny_jpeg().len());
        assert_eq!(asset.bytes, tiny_jpeg());
        assert!(!asset
            .bytes
            .windows(private_exif.len())
            .any(|window| window == private_exif));
        assert!(!asset
            .bytes
            .windows(private_comment.len())
            .any(|window| window == private_comment));

        let serialized = serde_json::to_string(&document).expect("document should serialize");
        assert!(!serialized.contains("private-client-photo"));
        assert!(!serialized.contains(dir.path().to_string_lossy().as_ref()));
        assert!(!serialized.contains("PRIVATE-EXIF"));
        assert!(!serialized.contains("PRIVATE-COMMENT"));
    }

    #[test]
    fn image_import_accepts_safe_tiny_jpeg_without_metadata() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let image_path = dir.path().join("local.jpeg");
        std::fs::write(&image_path, tiny_jpeg()).expect("jpeg should write");

        let (extension, media_type, bytes) =
            read_validated_image(&image_path).expect("safe jpeg should import");

        assert_eq!(extension, "jpg");
        assert_eq!(media_type, "image/jpeg");
        assert_eq!(bytes, tiny_jpeg());
    }

    #[test]
    fn image_import_rejects_malformed_metadata_jpeg_without_private_leak() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let image_path = dir.path().join("private-client-photo.jpg");
        let private_payload = b"PRIVATE-EXIF local/user/photo.jpg";
        let mut malformed = vec![0xff, 0xd8, 0xff, 0xe1, 0x00, 0x20];
        malformed.extend_from_slice(private_payload);
        std::fs::write(&image_path, malformed).expect("malformed jpeg should write");

        let err = read_validated_image(&image_path).expect_err("malformed jpeg should fail");

        assert_eq!(err, "image file is unsupported");
        assert!(!err.contains("PRIVATE-EXIF"));
        assert!(!err.contains("private-client-photo"));
        assert!(!err.contains(dir.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn image_import_rejects_post_scan_jpeg_metadata_without_private_leak() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let image_path = dir.path().join("private-client-photo.jpg");
        let private_payload = b"PRIVATE-COMMENT camera serial";
        let mut jpeg = tiny_jpeg();
        jpeg.truncate(jpeg.len() - 2);
        jpeg.extend_from_slice(&[0xff, 0xfe]);
        jpeg.extend_from_slice(&(private_payload.len() as u16 + 2).to_be_bytes());
        jpeg.extend_from_slice(private_payload);
        jpeg.extend_from_slice(&[0xff, 0xd9]);
        std::fs::write(&image_path, jpeg).expect("jpeg should write");

        let err = read_validated_image(&image_path).expect_err("post-scan metadata should fail");

        assert_eq!(err, "image file is unsupported");
        assert!(!err.contains("PRIVATE-COMMENT"));
        assert!(!err.contains("private-client-photo"));
        assert!(!err.contains(dir.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn image_import_undo_removes_block_and_embedded_asset() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let image_path = dir.path().join("logo.png");
        std::fs::write(&image_path, tiny_png()).expect("image should write");
        let mut session = DocumentSession::default();

        import_image_into_session(&mut session, &image_path, Some(0), Some(1))
            .expect("image import should succeed");

        let mut document = session.document.clone();
        session
            .undo
            .undo(&mut document)
            .expect("image import should undo as one change");

        assert!(document.assets.is_empty());
        assert_eq!(document.sections[0].blocks.len(), 1);
        assert!(!matches!(document.sections[0].blocks[0], Block::Image(_)));
    }

    #[test]
    fn image_import_appends_when_requested_index_is_out_of_range() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let image_path = dir.path().join("local.png");
        std::fs::write(&image_path, tiny_png()).expect("image should write");
        let mut session = DocumentSession::default();

        let document = import_image_into_session(&mut session, &image_path, Some(0), Some(99))
            .expect("image import should succeed");

        assert!(matches!(
            document.sections[0].blocks.last(),
            Some(Block::Image(_))
        ));
    }

    #[test]
    fn image_import_rejects_wrong_extension_without_filename_leak() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let image_path = dir.path().join("private-client-logo.txt");
        std::fs::write(&image_path, tiny_png()).expect("image should write");

        let err = read_validated_image(&image_path).expect_err("wrong extension should fail");

        assert_eq!(err, "image file is unsupported");
        assert!(!err.contains("private-client-logo"));
    }

    #[test]
    fn image_import_rejects_magic_extension_mismatch() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let image_path = dir.path().join("logo.jpg");
        std::fs::write(&image_path, tiny_png()).expect("image should write");

        let err = read_validated_image(&image_path).expect_err("mismatch should fail");

        assert_eq!(err, "image file is unsupported");
    }

    #[test]
    fn image_import_rejects_traversal_and_oversized_inputs() {
        let err = validate_image_path("../private-logo.png").expect_err("traversal should fail");
        assert_eq!(err, "image file is unsupported");

        let dir = tempfile::tempdir().expect("temp dir should be created");
        let image_path = dir.path().join("large.png");
        let mut bytes = tiny_png();
        bytes.resize(MAX_IMAGE_BYTES as usize + 1, 0);
        std::fs::write(&image_path, bytes).expect("image should write");

        let err = read_validated_image(&image_path).expect_err("oversized image should fail");
        assert_eq!(err, IMAGE_TOO_LARGE_ERROR);
        assert!(!err.contains("large.png"));
    }

    #[test]
    fn export_write_validates_extension_without_leaking_path() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let target = dir.path().join("private-client-name.txt");
        let result = write_export_bytes_to_path("txt", &target, b"hello")
            .expect("export write should succeed");

        assert_eq!(result.format, "txt");
        assert_eq!(result.byte_len, 5);
        assert_eq!(
            std::fs::read_to_string(&target).expect("export should exist"),
            "hello"
        );
    }

    #[test]
    fn docx_paths_validate_extension_without_leaking_path() {
        let err = validate_path("private-client-name.odt", "docx")
            .expect_err("odt path should fail for docx conversion");

        assert_eq!(err, "expected .docx document path");
        assert!(!err.contains("private-client-name"));
    }

    #[test]
    fn docx_export_write_returns_format_and_byte_count_only() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let target = dir.path().join("private-client-name.docx");
        let bytes = word_docx::write_docx_bytes(&Document::new_untitled())
            .expect("docx bytes should write");
        let result = write_export_bytes_to_path("docx", &target, &bytes)
            .expect("docx export write should succeed");

        assert_eq!(result.format, "docx");
        assert_eq!(result.byte_len, bytes.len() as u64);
        assert!(!result.format.contains("private-client-name"));
        word_docx::validate_docx_package(&bytes, word_docx::PackageLimits::default())
            .expect("exported docx package should validate");
    }

    #[test]
    fn docx_import_errors_do_not_leak_package_entry_names() {
        let err = safe_docx_import_error(word_docx::DocxError::UnsafePath {
            name: "C:/placeholder-private-document/document.xml".to_string(),
        });

        assert_eq!(err, "DOCX package contains unsupported or unsafe entries");
        assert!(!err.contains("placeholder-private-document"));
        assert!(!err.contains("document.xml"));
    }

    #[test]
    fn docx_export_errors_do_not_leak_internal_details() {
        let err = safe_docx_export_error(word_docx::DocxError::MissingDocument);

        assert_eq!(err, "DOCX export could not be prepared");
        assert!(!err.contains("document.xml"));
    }

    #[test]
    fn txt_export_path_rejects_wrong_extension_message_only() {
        let err = validate_path("private-client-name.html", "txt")
            .expect_err("wrong export extension should fail");

        assert_eq!(err, "expected .txt document path");
        assert!(!err.contains("private-client-name"));
    }

    #[test]
    fn rejects_parent_traversal() {
        let err = validate_path("../document.odt", "odt").expect_err("traversal should fail");

        assert_eq!(err, "path contains unsupported traversal");
    }

    #[test]
    fn recovery_tokens_reject_traversal_and_plain_paths() {
        assert!(
            validate_recovery_token("recovery-00000000-0000-4000-8000-000000000001.odt").is_ok()
        );
        assert!(validate_recovery_token(
            "recovery-v1-00000000000040008000000000000001-00000000000040008000000000000002.odt"
        )
        .is_ok());
        assert_eq!(
            validate_recovery_token("../private.odt").expect_err("traversal should fail"),
            "recovery token is invalid"
        );
        assert_eq!(
            validate_recovery_token("folder/recovery-private.odt").expect_err("path should fail"),
            "recovery token is invalid"
        );
        assert_eq!(
            validate_recovery_token("document.odt").expect_err("plain path should fail"),
            "recovery token is invalid"
        );
        assert_eq!(
            validate_recovery_token("recovery-document.odt")
                .expect_err("plain recovery name should fail"),
            "recovery token is invalid"
        );
    }

    #[test]
    fn recovery_token_generation_uses_versioned_opaque_components() {
        let mut document = Document::new_untitled();
        document.id = uuid::Uuid::from_u128(0x00000000_0000_4000_8000_000000000011);

        let token = recovery_token_for_document(&document);
        let parts = parse_recovery_token(&token).expect("generated token should validate");

        assert!(token.starts_with("recovery-v1-"));
        assert!(token.ends_with(".odt"));
        assert_eq!(parts.document_key, "00000000000040008000000000000011");
        assert!(!token.contains("Untitled"));
        assert!(!token.contains('/'));
        assert!(!token.contains('\\'));
    }

    #[test]
    fn recovery_autosave_writes_versioned_snapshots_and_bounds_per_document() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let mut document = Document::new_untitled();
        document.id = uuid::Uuid::from_u128(0x00000000_0000_4000_8000_000000000021);

        let mut written_tokens = HashSet::new();
        for _ in 0..(RECOVERY_SNAPSHOTS_PER_DOCUMENT + 2) {
            let summary = write_recovery_document_in_dir(&document, dir.path())
                .expect("recovery snapshot should write");
            assert!(summary.token.starts_with("recovery-v1-"));
            assert!(summary.label.starts_with("Recovery draft "));
            assert!(!summary.label.contains("Untitled"));
            assert!(!summary.token.contains("Untitled"));
            written_tokens.insert(summary.token);
        }

        let summaries =
            list_recovery_documents_in_dir(dir.path()).expect("recovery snapshots should list");
        assert_eq!(summaries.len(), RECOVERY_SNAPSHOTS_PER_DOCUMENT);
        assert!(written_tokens.len() > RECOVERY_SNAPSHOTS_PER_DOCUMENT);
        for summary in summaries {
            assert!(summary.token.starts_with("recovery-v1-"));
            assert!(summary.label.starts_with("Recovery draft "));
            assert!(summary.byte_len > 0);
            assert!(!summary.label.contains("Untitled"));
        }
    }

    #[test]
    fn recovery_autosave_bounds_total_snapshot_count() {
        let dir = tempfile::tempdir().expect("temp dir should be created");

        for index in 0..(MAX_RECOVERY_SNAPSHOTS + 5) {
            let mut document = Document::new_untitled();
            document.id =
                uuid::Uuid::from_u128(0x00000000_0000_4000_8000_000000000100 + index as u128);
            write_recovery_document_in_dir(&document, dir.path())
                .expect("recovery snapshot should write");
        }

        let summaries =
            list_recovery_documents_in_dir(dir.path()).expect("recovery snapshots should list");
        assert_eq!(summaries.len(), MAX_RECOVERY_SNAPSHOTS);
        assert!(summaries
            .iter()
            .all(|summary| summary.label.starts_with("Recovery draft ")));
    }

    #[test]
    fn legacy_recovery_tokens_still_list_and_recover() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let mut document = Document::new_untitled();
        document.id = uuid::Uuid::from_u128(0x00000000_0000_4000_8000_000000000031);
        let legacy_token = format!("recovery-{}.odt", document.id);
        let legacy_path = recovery_path_for_token_in_dir(&legacy_token, dir.path())
            .expect("legacy token should produce path");
        let bytes = word_odf::write_odt_bytes(&document).expect("document should write as ODT");
        ensure_private_recovery_dir_at(dir.path()).expect("recovery dir should be private");
        write_bytes_atomically(&legacy_path, &bytes, true).expect("legacy recovery should write");

        let summaries =
            list_recovery_documents_in_dir(dir.path()).expect("legacy recovery should list");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].token, legacy_token);
        assert_eq!(summaries[0].label, "Recovery draft 1");
        assert!(!summaries[0].label.contains("Untitled"));

        let mut session = DocumentSession::default();
        session.current_path = Some(PathBuf::from("document.odt"));
        session.dirty = false;
        let recovered = recover_document_from_dir(&legacy_token, dir.path(), &mut session)
            .expect("legacy recovery should recover");

        assert_eq!(recovered.meta.title, "Untitled Document");
        assert!(session.dirty);
        assert!(session.current_path.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_recovery_entries_are_not_listed_or_opened() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().expect("temp dir should be created");
        let external = tempfile::tempdir().expect("external temp dir should be created");
        let document = Document::new_untitled();
        let bytes = word_odf::write_odt_bytes(&document).expect("document should write as ODT");
        let target = external.path().join("target.odt");
        std::fs::write(&target, bytes).expect("external target should write");

        let token =
            "recovery-v1-00000000000040008000000000000061-00000000000040008000000000000062.odt";
        let symlink_path = dir.path().join(token);
        symlink(&target, &symlink_path).expect("recovery symlink should write");

        let summaries =
            list_recovery_documents_in_dir(dir.path()).expect("recovery list should succeed");
        assert!(summaries.is_empty());

        let mut session = DocumentSession::default();
        assert_eq!(
            recover_document_from_dir(token, dir.path(), &mut session)
                .expect_err("recovery symlink should not be opened"),
            "recovery token is invalid"
        );
        assert!(!session.dirty);
        assert!(session.current_path.is_none());
    }

    #[test]
    fn recovery_open_keeps_draft_dirty_and_unsaved() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let mut document = Document::new_untitled();
        document.id = uuid::Uuid::from_u128(0x00000000_0000_4000_8000_000000000041);
        let summary = write_recovery_document_in_dir(&document, dir.path())
            .expect("recovery snapshot should write");
        let mut session = DocumentSession::default();
        session.current_path = Some(PathBuf::from("document.odt"));
        session.dirty = false;

        let recovered = recover_document_from_dir(&summary.token, dir.path(), &mut session)
            .expect("recovery snapshot should open");

        assert_eq!(recovered.meta.title, "Untitled Document");
        assert!(session.dirty);
        assert!(session.current_path.is_none());
    }

    #[test]
    fn recovery_discard_is_scoped_to_selected_validated_token() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let mut first = Document::new_untitled();
        first.id = uuid::Uuid::from_u128(0x00000000_0000_4000_8000_000000000051);
        let mut second = Document::new_untitled();
        second.id = uuid::Uuid::from_u128(0x00000000_0000_4000_8000_000000000052);
        let first_summary = write_recovery_document_in_dir(&first, dir.path())
            .expect("first recovery snapshot should write");
        let second_summary = write_recovery_document_in_dir(&second, dir.path())
            .expect("second recovery snapshot should write");
        let plain_path = dir.path().join("document.odt");
        std::fs::write(&plain_path, b"not a recovery token").expect("plain file should write");

        discard_recovery_from_dir(&first_summary.token, dir.path())
            .expect("selected recovery should discard");

        assert!(
            !recovery_path_for_token_in_dir(&first_summary.token, dir.path())
                .expect("first token should stay valid")
                .exists()
        );
        assert!(
            recovery_path_for_token_in_dir(&second_summary.token, dir.path())
                .expect("second token should stay valid")
                .exists()
        );
        assert!(plain_path.exists());
        assert_eq!(
            discard_recovery_from_dir("../document.odt", dir.path())
                .expect_err("traversal discard should fail"),
            "recovery token is invalid"
        );
        assert_eq!(
            discard_recovery_from_dir("document.odt", dir.path())
                .expect_err("plain discard should fail"),
            "recovery token is invalid"
        );
    }

    #[test]
    fn recent_summaries_do_not_expose_paths_or_filenames() {
        let mut session = DocumentSession::default();
        let private_path = PathBuf::from("private-client-name.odt");

        remember_recent_path(&mut session, private_path.clone());
        session.current_path = Some(private_path);

        let summaries = recent_summaries(&session);

        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].is_current);
        assert_eq!(summaries[0].label, "Recent document 1");
        assert_eq!(summaries[0].token, "recent-1");
        assert!(!summaries[0].label.contains("private-client-name"));
        assert!(!summaries[0].token.contains("private-client-name"));
    }

    #[test]
    fn file_state_reports_dirty_without_path_details() {
        let mut session = DocumentSession::default();
        session.dirty = true;
        remember_recent_path(&mut session, PathBuf::from("private-draft.odt"));

        let state = DocumentFileState {
            has_current_path: session.current_path.is_some(),
            dirty: session.dirty,
            recent_documents: recent_summaries(&session),
            recovery_documents: Vec::new(),
        };

        assert!(state.dirty);
        assert!(!state.has_current_path);
        assert_eq!(state.recent_documents[0].label, "Recent document 1");
    }

    #[test]
    fn templates_are_generated_and_generic() {
        let summaries = template_summaries();

        let expected_ids = vec![
            "blank",
            "report",
            "letter",
            "project-report",
            "resume",
            "meeting-minutes",
            "memo",
            "invoice",
            "flyer",
        ];
        assert_eq!(
            summaries
                .iter()
                .map(|template| template.id.as_str())
                .collect::<Vec<_>>(),
            expected_ids
        );

        let denied_fragments = [
            "private",
            "/Users/",
            "/Volumes/",
            "../",
            "C:\\",
            "file:",
            "localhost",
            "127.0.0.1",
            "900Labs",
            "samir",
            "john",
            "jane",
            "acme",
            "globex",
        ];
        for summary in summaries {
            assert!(!summary.name.trim().is_empty());
            assert!(!summary.description.trim().is_empty());

            let document = build_template_document(&summary.id)
                .unwrap_or_else(|_| panic!("{} template should exist", summary.id));
            let serialized =
                serde_json::to_string(&document).expect("template should serialize to JSON");
            let exported_text =
                word_export::export_txt(&document).expect("template text should export");
            let combined = format!(
                "{}\n{}\n{}\n{}",
                summary.name, summary.description, serialized, exported_text
            )
            .to_lowercase();

            for denied_fragment in denied_fragments {
                assert!(
                    !combined.contains(&denied_fragment.to_lowercase()),
                    "{} template should not contain {}",
                    summary.id,
                    denied_fragment
                );
            }
        }
    }

    #[test]
    fn generated_templates_round_trip_through_odt() {
        for summary in template_summaries() {
            let document = build_template_document(&summary.id)
                .unwrap_or_else(|_| panic!("{} template should exist", summary.id));
            let bytes = word_odf::write_odt_bytes(&document)
                .unwrap_or_else(|_| panic!("{} template should write as odt", summary.id));
            let parsed = word_odf::read_odt_bytes(&bytes)
                .unwrap_or_else(|_| panic!("{} template should reopen as odt", summary.id));

            assert_eq!(parsed.meta.title, document.meta.title);
            assert_eq!(
                parsed.sections[0].blocks.len(),
                document.sections[0].blocks.len()
            );
        }
    }

    #[test]
    fn table_heavy_templates_include_table_blocks() {
        let invoice = build_template_document("invoice").expect("invoice template should exist");
        let project_report =
            build_template_document("project-report").expect("project template should exist");

        assert!(document_has_table(&invoice));
        assert!(document_has_table(&project_report));
    }

    #[test]
    fn unknown_template_is_rejected_without_path_handling() {
        for template_id in ["unknown", "../report", "/tmp/report", "C:\\draft\\report"] {
            let err =
                build_template_document(template_id).expect_err("unknown template id should fail");

            assert_eq!(err, "template is unavailable");
            assert!(!err.contains(template_id));
        }
    }

    fn document_has_table(document: &Document) -> bool {
        document.sections.iter().any(|section| {
            section
                .blocks
                .iter()
                .any(|block| matches!(block, Block::Table(_)))
        })
    }

    #[test]
    fn atomic_temp_path_does_not_copy_target_filename() {
        let target = PathBuf::from("private-client-name.odt");

        let temp_path = atomic_temp_path(&target);
        let temp_name = temp_path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("temp path should have a valid filename");

        assert!(temp_name.starts_with(".900word-write-"));
        assert!(temp_name.ends_with(".tmp"));
        assert!(!temp_name.contains("private-client-name"));
    }

    #[test]
    fn atomic_write_rejects_oversized_output_before_creating_target() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let target = dir.path().join("document.odt");
        let bytes = vec![0_u8; MAX_DOCUMENT_BYTES as usize + 1];

        let err = write_bytes_atomically(&target, &bytes, false)
            .expect_err("oversized output should fail");

        assert_eq!(err, "document exceeds supported bootstrap size limit");
        assert!(!target.exists());
    }

    #[cfg(unix)]
    #[test]
    fn recovery_write_uses_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("temp dir should be created");
        let document = Document::new_untitled();

        let summary = write_recovery_document_in_dir(&document, dir.path())
            .expect("recovery snapshot should write");
        let path = recovery_path_for_token_in_dir(&summary.token, dir.path())
            .expect("recovery token should produce path");

        let dir_mode = std::fs::metadata(dir.path())
            .expect("dir metadata should exist")
            .permissions()
            .mode()
            & 0o777;
        let file_mode = std::fs::metadata(path)
            .expect("file metadata should exist")
            .permissions()
            .mode()
            & 0o777;

        assert_eq!(dir_mode, 0o700);
        assert_eq!(file_mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn private_atomic_write_uses_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("temp dir should be created");
        let target = dir.path().join("recovery.odt");

        write_bytes_atomically(&target, b"private recovery bytes", true)
            .expect("private write should succeed");

        let dir_mode = std::fs::metadata(dir.path())
            .expect("dir metadata should exist")
            .permissions()
            .mode()
            & 0o777;
        let file_mode = std::fs::metadata(&target)
            .expect("file metadata should exist")
            .permissions()
            .mode()
            & 0o777;

        assert_eq!(dir_mode, 0o700);
        assert_eq!(file_mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn normal_atomic_write_preserves_existing_private_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("temp dir should be created");
        let target = dir.path().join("document.odt");
        std::fs::write(&target, b"old document").expect("seed file should write");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600))
            .expect("seed permissions should apply");

        write_bytes_atomically(&target, b"new document", false).expect("normal write should pass");

        let mode = std::fs::metadata(&target)
            .expect("file metadata should exist")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn normal_atomic_write_rejects_read_only_target() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("temp dir should be created");
        let target = dir.path().join("document.odt");
        std::fs::write(&target, b"old document").expect("seed file should write");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o400))
            .expect("seed permissions should apply");

        let err = write_bytes_atomically(&target, b"new document", false)
            .expect_err("read-only target should fail");

        assert_eq!(err, "target document is read-only");
        assert_eq!(
            std::fs::read(&target).expect("target should remain readable"),
            b"old document"
        );
        let mode = std::fs::metadata(&target)
            .expect("file metadata should exist")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o400);
    }

    #[test]
    fn settings_never_enable_telemetry() {
        let settings = sanitize_settings(Settings {
            telemetry_enabled: true,
            language_tag: "en".to_string(),
            ui_locale: "unknown".to_string(),
            high_contrast: true,
            large_toolbar: true,
            reduced_motion: true,
            low_resource: true,
            smart_typing: SmartTypingSettings {
                capitalize_sentences: true,
                smart_quotes: true,
                smart_dashes: true,
                typo_replacements: true,
                list_triggers: true,
            },
        });

        assert!(!settings.telemetry_enabled);
        assert_eq!(settings.language_tag, "en");
        assert_eq!(settings.ui_locale, "en-US");
        assert!(settings.high_contrast);
        assert!(settings.large_toolbar);
        assert!(settings.reduced_motion);
        assert!(settings.low_resource);
        assert!(settings.smart_typing.capitalize_sentences);
        assert!(settings.smart_typing.smart_quotes);
        assert!(settings.smart_typing.smart_dashes);
        assert!(settings.smart_typing.typo_replacements);
        assert!(settings.smart_typing.list_triggers);
    }

    #[test]
    fn persisted_settings_keep_telemetry_disabled_after_save_and_load() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let path = dir.path().join("settings.json");

        let saved = save_settings_to_path(
            &path,
            Settings {
                telemetry_enabled: true,
                language_tag: "en_US".to_string(),
                ui_locale: "es-ES".to_string(),
                high_contrast: true,
                large_toolbar: false,
                reduced_motion: true,
                low_resource: false,
                smart_typing: SmartTypingSettings::default(),
            },
        )
        .expect("settings should save");
        let loaded = load_settings_from_path(&path);

        assert!(!saved.telemetry_enabled);
        assert!(!loaded.telemetry_enabled);
        assert_eq!(loaded.language_tag, "en-US");
        assert_eq!(loaded.ui_locale, "es-ES");
        assert!(loaded.high_contrast);
        assert!(loaded.reduced_motion);
        let raw = std::fs::read_to_string(&path).expect("settings should be readable");
        assert!(raw.contains("\"telemetry_enabled\": false"));
    }

    #[test]
    fn settings_round_trip_through_temp_settings_path() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let path = dir.path().join("settings.json");

        save_settings_to_path(
            &path,
            Settings {
                telemetry_enabled: false,
                language_tag: "de-DE".to_string(),
                ui_locale: "ar".to_string(),
                high_contrast: true,
                large_toolbar: true,
                reduced_motion: true,
                low_resource: true,
                smart_typing: SmartTypingSettings {
                    capitalize_sentences: true,
                    smart_quotes: true,
                    smart_dashes: false,
                    typo_replacements: true,
                    list_triggers: false,
                },
            },
        )
        .expect("settings should save");

        let loaded = load_settings_from_path(&path);
        assert_eq!(loaded.language_tag, "de-DE");
        assert_eq!(loaded.ui_locale, "ar");
        assert!(loaded.high_contrast);
        assert!(loaded.large_toolbar);
        assert!(loaded.reduced_motion);
        assert!(loaded.low_resource);
        assert!(loaded.smart_typing.capitalize_sentences);
        assert!(loaded.smart_typing.smart_quotes);
        assert!(!loaded.smart_typing.smart_dashes);
        assert!(loaded.smart_typing.typo_replacements);
        assert!(!loaded.smart_typing.list_triggers);
    }

    #[test]
    fn reset_settings_rewrites_sanitized_defaults() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let path = dir.path().join("settings.json");
        save_settings_to_path(
            &path,
            Settings {
                telemetry_enabled: true,
                language_tag: "de-DE".to_string(),
                ui_locale: "es-ES".to_string(),
                high_contrast: true,
                large_toolbar: true,
                reduced_motion: true,
                low_resource: true,
                smart_typing: SmartTypingSettings {
                    capitalize_sentences: true,
                    smart_quotes: true,
                    smart_dashes: true,
                    typo_replacements: true,
                    list_triggers: true,
                },
            },
        )
        .expect("settings should save");

        let reset = reset_settings_at_path(&path).expect("settings should reset");
        let loaded = load_settings_from_path(&path);
        let raw = std::fs::read_to_string(&path).expect("settings should be readable");

        assert!(!reset.telemetry_enabled);
        assert_eq!(reset.language_tag, FALLBACK_LANGUAGE_TAG);
        assert_eq!(reset.ui_locale, "en-US");
        assert!(!reset.high_contrast);
        assert!(!reset.large_toolbar);
        assert!(!reset.reduced_motion);
        assert!(!reset.low_resource);
        assert!(!reset.smart_typing.capitalize_sentences);
        assert!(!reset.smart_typing.smart_quotes);
        assert!(!reset.smart_typing.smart_dashes);
        assert!(!reset.smart_typing.typo_replacements);
        assert!(!reset.smart_typing.list_triggers);
        assert!(!loaded.telemetry_enabled);
        assert_eq!(loaded.language_tag, FALLBACK_LANGUAGE_TAG);
        assert_eq!(loaded.ui_locale, "en-US");
        assert!(raw.contains("\"telemetry_enabled\": false"));
        assert!(!raw.contains("de-DE"));
        assert!(!raw.contains("es-ES"));
    }

    #[test]
    fn reset_settings_handles_missing_file() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let path = dir.path().join("settings.json");

        let reset = reset_settings_at_path(&path).expect("missing settings should reset");

        assert!(path.exists());
        assert!(!reset.telemetry_enabled);
        assert_eq!(reset.language_tag, FALLBACK_LANGUAGE_TAG);
        assert_eq!(reset.ui_locale, "en-US");
        assert!(!reset.high_contrast);
    }

    #[test]
    fn reset_settings_failure_does_not_leak_path_details() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let path = dir.path().join("private-client-settings.json");
        std::fs::create_dir(&path).expect("settings directory should be created");

        let err = reset_settings_at_path(&path).expect_err("directory target should fail");

        assert_eq!(err, "settings could not be saved");
        assert!(!err.contains(dir.path().to_string_lossy().as_ref()));
        assert!(!err.contains("private-client-settings"));
        assert!(!err.contains("settings.json"));
    }

    #[test]
    fn reset_settings_unsafe_parent_does_not_leak_path_details() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let parent = dir.path().join("private-client-home");
        std::fs::write(&parent, b"not a directory").expect("parent file should write");
        let path = parent.join("settings.json");

        let err = reset_settings_at_path(&path).expect_err("unsafe parent should fail");

        assert_eq!(err, "settings storage is unavailable");
        assert!(!err.contains(dir.path().to_string_lossy().as_ref()));
        assert!(!err.contains("private-client-home"));
        assert!(!err.contains("settings.json"));
    }

    #[cfg(unix)]
    #[test]
    fn reset_settings_rewrite_uses_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("temp dir should exist");
        let path = dir.path().join("settings.json");
        std::fs::write(&path, b"{\"telemetry_enabled\":true}").expect("settings seed should write");
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755))
            .expect("dir permissions should apply");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .expect("file permissions should apply");

        reset_settings_at_path(&path).expect("settings should reset");

        let dir_mode = std::fs::metadata(dir.path())
            .expect("dir metadata should exist")
            .permissions()
            .mode()
            & 0o777;
        let file_mode = std::fs::metadata(&path)
            .expect("settings metadata should exist")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o700);
        assert_eq!(file_mode, 0o600);
    }

    #[test]
    fn malformed_settings_fall_back_to_defaults_without_path_details() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let path = dir.path().join("private-client-settings.json");
        std::fs::write(&path, "{ not valid settings json")
            .expect("malformed settings should write");

        let loaded = load_settings_from_path(&path);
        let err = try_load_settings_from_path(&path).expect_err("malformed settings should error");

        assert!(!loaded.telemetry_enabled);
        assert_eq!(loaded.language_tag, FALLBACK_LANGUAGE_TAG);
        assert_eq!(loaded.ui_locale, "en-US");
        assert_eq!(err, "settings could not be read");
        assert!(!err.contains(dir.path().to_string_lossy().as_ref()));
        assert!(!err.contains("private-client-settings"));
    }

    #[test]
    fn oversized_settings_file_falls_back_to_defaults() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let path = dir.path().join("settings.json");
        std::fs::write(&path, vec![b' '; MAX_SETTINGS_BYTES as usize + 1])
            .expect("oversized settings should write");

        let loaded = load_settings_from_path(&path);
        let direct =
            try_load_settings_from_path(&path).expect("oversized settings should load default");

        assert!(!loaded.telemetry_enabled);
        assert_eq!(loaded.language_tag, FALLBACK_LANGUAGE_TAG);
        assert_eq!(loaded.ui_locale, "en-US");
        assert!(!direct.telemetry_enabled);
        assert_eq!(direct.language_tag, FALLBACK_LANGUAGE_TAG);
        assert_eq!(direct.ui_locale, "en-US");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_settings_file_falls_back_to_defaults() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().expect("temp dir should exist");
        let target = dir.path().join("real-settings.json");
        let symlink_path = dir.path().join("settings.json");
        save_settings_to_path(
            &target,
            Settings {
                telemetry_enabled: true,
                language_tag: "de-DE".to_string(),
                ui_locale: "es-ES".to_string(),
                high_contrast: true,
                large_toolbar: true,
                reduced_motion: true,
                low_resource: true,
                smart_typing: SmartTypingSettings {
                    capitalize_sentences: true,
                    smart_quotes: true,
                    smart_dashes: true,
                    typo_replacements: true,
                    list_triggers: true,
                },
            },
        )
        .expect("target settings should save");
        symlink(&target, &symlink_path).expect("settings symlink should write");

        let loaded = load_settings_from_path(&symlink_path);
        let direct = try_load_settings_from_path(&symlink_path)
            .expect("symlinked settings should load default");

        assert_eq!(loaded.language_tag, FALLBACK_LANGUAGE_TAG);
        assert_eq!(loaded.ui_locale, "en-US");
        assert!(!loaded.high_contrast);
        assert_eq!(direct.language_tag, FALLBACK_LANGUAGE_TAG);
        assert_eq!(direct.ui_locale, "en-US");
        assert!(!direct.high_contrast);
    }

    #[test]
    fn path_like_language_settings_fall_back_before_save() {
        let settings = sanitize_settings(Settings {
            telemetry_enabled: false,
            language_tag: "folder/private-draft.odt".to_string(),
            ui_locale: "settings/private".to_string(),
            high_contrast: false,
            large_toolbar: false,
            reduced_motion: false,
            low_resource: false,
            smart_typing: SmartTypingSettings::default(),
        });

        assert_eq!(settings.language_tag, FALLBACK_LANGUAGE_TAG);
        assert_eq!(settings.ui_locale, "en-US");
    }

    #[cfg(unix)]
    #[test]
    fn settings_save_uses_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("temp dir should exist");
        let path = dir.path().join("settings.json");

        save_settings_to_path(&path, Settings::default()).expect("settings should save");

        let dir_mode = std::fs::metadata(dir.path())
            .expect("dir metadata should exist")
            .permissions()
            .mode()
            & 0o777;
        let file_mode = std::fs::metadata(&path)
            .expect("settings metadata should exist")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o700);
        assert_eq!(file_mode, 0o600);
    }

    #[test]
    fn missing_dictionary_falls_back_without_path_details() {
        let dir = tempfile::tempdir().expect("temp dir should exist");

        let result = check_spelling_with_root("hello qwerty", "zz-ZZ", dir.path())
            .expect("fallback check should succeed");

        assert_eq!(result.language_tag, "en-US");
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].word, "qwerty");
        assert_eq!(result.warnings.len(), 1);
        assert!(!result.warnings[0].contains(dir.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn user_dictionary_check_uses_sanitized_root_boundary() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        std::fs::write(dir.path().join("de-DE.aff"), "SET UTF-8\n").expect("aff should write");
        std::fs::write(dir.path().join("de-DE.dic"), "2\nhallo\ndokument\n")
            .expect("dic should write");

        let result = check_spelling_with_root("hallo dokument", "de-DE", dir.path())
            .expect("user dictionary check should succeed");

        assert_eq!(result.language_tag, "de-DE");
        assert!(result.issues.is_empty());
    }

    #[test]
    fn personal_dictionary_words_are_used_by_spell_check() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        word_spell::add_personal_word(dir.path(), "en-US", "qwerty")
            .expect("personal word should write");

        let result = check_spelling_with_root("hello qwerty", "en-US", dir.path())
            .expect("personal dictionary check should succeed");

        assert!(result.issues.is_empty());
    }

    #[test]
    fn personal_dictionary_words_list_plain_normalized_words() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        word_spell::add_personal_word(dir.path(), "en-US", "Qwerty")
            .expect("personal word should write");
        word_spell::add_personal_word(dir.path(), "en-US", "Alpha")
            .expect("personal word should write");

        let words = list_personal_dictionary_words_with_root("en-US", dir.path())
            .expect("personal words should list");

        assert_eq!(words, vec!["alpha".to_string(), "qwerty".to_string()]);
        assert!(words
            .iter()
            .all(|word| !word.contains('/') && !word.contains('\\')));
    }

    #[test]
    fn personal_dictionary_words_missing_file_is_empty() {
        let dir = tempfile::tempdir().expect("temp dir should exist");

        let words = list_personal_dictionary_words_with_root("en-US", dir.path())
            .expect("missing personal words should be empty");

        assert!(words.is_empty());
    }

    #[test]
    fn removing_personal_dictionary_word_updates_spell_check_boundary() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        word_spell::add_personal_word(dir.path(), "en-US", "qwerty")
            .expect("personal word should write");

        let remaining = remove_personal_dictionary_word_with_root("qwerty", "en-US", dir.path())
            .expect("personal word should remove");
        let result = check_spelling_with_root("hello qwerty", "en-US", dir.path())
            .expect("spell check should still run");

        assert!(remaining.is_empty());
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].word, "qwerty");
    }

    #[test]
    fn removing_invalid_personal_dictionary_word_returns_generic_error() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let invalid_word = "!";

        let err = remove_personal_dictionary_word_with_root(invalid_word, "en-US", dir.path())
            .expect_err("invalid personal word should fail");

        assert_eq!(err, "personal dictionary word is invalid");
        assert!(!err.contains(invalid_word));
        assert!(!err.contains(dir.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn invalid_personal_dictionary_language_returns_generic_error() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let invalid_language = "!";

        let err = list_personal_dictionary_words_with_root(invalid_language, dir.path())
            .expect_err("bad tag should fail");

        assert_eq!(err, "personal dictionary is unavailable");
        assert!(!err.contains(invalid_language));
        assert!(!err.contains(dir.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn install_user_dictionary_helper_returns_installed_dictionary() {
        let user_dir = tempfile::tempdir().expect("user dir should exist");
        let source_dir = tempfile::tempdir().expect("source dir should exist");
        write_test_dictionary(source_dir.path(), "sv-SE", &["hej", "dokument"]);

        let installed = install_user_dictionary_with_root(
            "sv-SE",
            source_dir
                .path()
                .join("sv-SE.aff")
                .to_string_lossy()
                .as_ref(),
            source_dir
                .path()
                .join("sv-SE.dic")
                .to_string_lossy()
                .as_ref(),
            user_dir.path(),
        )
        .expect("dictionary should install");

        assert_eq!(installed.language_tag, "sv-SE");
        assert!(installed.user);
        let dictionaries = word_spell::list_dictionaries_with_user_root(user_dir.path());
        assert!(dictionaries
            .iter()
            .any(|dictionary| dictionary.language_tag == "sv-SE" && dictionary.user));
    }

    #[test]
    fn install_user_dictionary_helper_sanitizes_invalid_language_error() {
        let user_dir = tempfile::tempdir().expect("user dir should exist");
        let source_dir = tempfile::tempdir().expect("source dir should exist");
        write_test_dictionary(source_dir.path(), "sv-SE", &["hej"]);
        let invalid_language = "privateclient";

        let err = install_user_dictionary_with_root(
            invalid_language,
            source_dir
                .path()
                .join("sv-SE.aff")
                .to_string_lossy()
                .as_ref(),
            source_dir
                .path()
                .join("sv-SE.dic")
                .to_string_lossy()
                .as_ref(),
            user_dir.path(),
        )
        .expect_err("invalid language should fail");

        assert_eq!(err, "invalid language");
        assert!(!err.contains(invalid_language));
        assert!(!err.contains(user_dir.path().to_string_lossy().as_ref()));
        assert!(!err.contains(source_dir.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn install_user_dictionary_helper_sanitizes_source_path_errors() {
        let user_dir = tempfile::tempdir().expect("user dir should exist");
        let private_path = "private-client-name.txt";

        let err = install_user_dictionary_with_root(
            "sv-SE",
            private_path,
            "dictionary.dic",
            user_dir.path(),
        )
        .expect_err("wrong extension should fail");

        assert_eq!(err, "unsupported file");
        assert!(!err.contains(private_path));
        assert!(!err.contains(user_dir.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn remove_user_dictionary_helper_removes_local_pair() {
        let user_dir = tempfile::tempdir().expect("user dir should exist");
        write_test_dictionary(user_dir.path(), "sv-SE", &["hej", "dokument"]);

        remove_user_dictionary_with_root("sv-SE", user_dir.path())
            .expect("dictionary should remove");

        assert!(!user_dir.path().join("sv-SE.aff").exists());
        assert!(!user_dir.path().join("sv-SE.dic").exists());
        assert!(
            word_spell::list_dictionaries_with_user_root(user_dir.path())
                .iter()
                .all(|dictionary| dictionary.language_tag != "sv-SE")
        );
    }

    #[test]
    fn remove_user_dictionary_helper_sanitizes_invalid_language_error() {
        let user_dir = tempfile::tempdir().expect("user dir should exist");
        let invalid_language = "privateclient";

        let err = remove_user_dictionary_with_root(invalid_language, user_dir.path())
            .expect_err("invalid language should fail");

        assert_eq!(err, "invalid language");
        assert!(!err.contains(invalid_language));
        assert!(!err.contains(user_dir.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn remove_user_dictionary_helper_keeps_bundled_fallback_available() {
        let user_dir = tempfile::tempdir().expect("user dir should exist");
        write_test_dictionary(user_dir.path(), "en-US", &["localonly"]);

        remove_user_dictionary_with_root("en-US", user_dir.path())
            .expect("user override should remove");

        let result = check_spelling_with_root("localonly", "en-US", user_dir.path())
            .expect("bundled fallback should check");
        assert_eq!(result.language_tag, "en-US");
        assert_eq!(result.issues.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn removing_personal_dictionary_word_preserves_owner_only_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("temp dir should exist");
        word_spell::add_personal_word(dir.path(), "en-US", "qwerty")
            .expect("personal word should write");
        word_spell::add_personal_word(dir.path(), "en-US", "zebra")
            .expect("personal word should write");
        let path = only_regular_file_in(dir.path());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .expect("test permissions should apply");

        remove_personal_dictionary_word_with_root("qwerty", "en-US", dir.path())
            .expect("personal word should remove");

        let mode = std::fs::metadata(&path)
            .expect("personal dictionary metadata should exist")
            .permissions()
            .mode()
            & 0o777;

        assert_eq!(mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn user_dictionary_dir_is_owner_only_when_created() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("temp dir should exist");
        let dictionary_dir = dir.path().join("dictionaries");

        ensure_user_dictionary_dir(&dictionary_dir).expect("dictionary dir should be created");

        let mode = std::fs::metadata(&dictionary_dir)
            .expect("dictionary dir metadata should exist")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700);
    }

    #[cfg(unix)]
    #[test]
    fn user_dictionary_dir_rejects_symlink_root() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().expect("temp dir should exist");
        let external = tempfile::tempdir().expect("external temp dir should exist");
        let dictionary_dir = dir.path().join("dictionaries");
        symlink(external.path(), &dictionary_dir).expect("symlink should write");

        let err = ensure_user_dictionary_dir(&dictionary_dir)
            .expect_err("dictionary dir symlink should fail");

        assert_eq!(err, "dictionary directory is unavailable");
    }

    #[test]
    fn frontend_startup_sources_do_not_use_network_primitives() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let startup_files = [
            "../src/App.svelte",
            "../src/main.ts",
            "../src/lib/editor.ts",
            "../src/lib/documentProjection.ts",
        ];
        let blocked_tokens = [
            "fetch(",
            "XMLHttpRequest",
            "WebSocket",
            "EventSource",
            "sendBeacon",
        ];

        for file in startup_files {
            let source = std::fs::read_to_string(manifest_dir.join(file))
                .unwrap_or_else(|error| panic!("failed to read {file}: {error}"));
            for token in blocked_tokens {
                assert!(
                    !source.contains(token),
                    "{file} must not use startup network primitive {token}"
                );
            }
        }
    }

    #[test]
    fn track_changes_privacy_and_format_docs_state_boundaries() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let privacy = std::fs::read_to_string(manifest_dir.join("../../../docs/PRIVACY.md"))
            .expect("privacy doc must be readable");
        let privacy_model =
            std::fs::read_to_string(manifest_dir.join("../../../docs/PRIVACY_MODEL.md"))
                .expect("privacy model doc must be readable");
        let file_formats =
            std::fs::read_to_string(manifest_dir.join("../../../docs/FILE_FORMATS.md"))
                .expect("file formats doc must be readable");

        assert!(privacy.contains("Tracked changes can reveal edit history and deleted text"));
        assert!(
            privacy.contains("local table-of-contents generation from supported document headings")
        );
        assert!(privacy_model.contains("The default author string is `Local User`"));
        assert!(privacy_model.contains("no operating-system username"));
        assert!(
            privacy_model.contains("generated bookmark IDs are compact document-local identifiers")
        );
        assert!(file_formats.contains("900Word-authored text-only tracked changes"));
        assert!(file_formats.contains("generated table-of-contents blocks"));
        assert!(file_formats.contains("`word900` metadata"));
        assert!(file_formats.contains("simple DOCX insertion/deletion conversion"));
        assert!(file_formats.contains("full Word review fidelity"));
    }

    #[test]
    fn default_capability_keeps_shell_access_out_of_core() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let capability = std::fs::read_to_string(manifest_dir.join("capabilities/default.json"))
            .expect("default capability must be readable");

        assert!(capability.contains("\"core:default\""));
        assert!(!capability.contains("shell"));
        assert!(!capability.contains("http:"));
        assert!(!capability.contains("https:"));
    }

    fn tiny_png() -> Vec<u8> {
        vec![
            0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n', 0x00, 0x00, 0x00, 0x0d, b'I', b'H',
            b'D', b'R', 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1f, 0x15, 0xc4, 0x89,
        ]
    }

    fn tiny_jpeg() -> Vec<u8> {
        vec![
            0xff, 0xd8, 0xff, 0xdb, 0x00, 0x04, 0x00, 0x00, 0xff, 0xc0, 0x00, 0x0b, 0x08, 0x00,
            0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xff, 0xda, 0x00, 0x08, 0x01, 0x01, 0x00,
            0x00, 0x3f, 0x00, 0x11, 0x22, 0xff, 0x00, 0x33, 0xff, 0xd9,
        ]
    }

    fn jpeg_with_metadata_segment(jpeg: &[u8], marker: u8, payload: &[u8]) -> Vec<u8> {
        assert!(jpeg.starts_with(b"\xff\xd8"));
        assert!((0xe0..=0xef).contains(&marker) || marker == 0xfe);
        let segment_length = payload.len() + 2;
        assert!(segment_length <= u16::MAX as usize);

        let mut output = Vec::with_capacity(jpeg.len() + payload.len() + 4);
        output.extend_from_slice(&jpeg[..2]);
        output.extend_from_slice(&[0xff, marker]);
        output.extend_from_slice(&(segment_length as u16).to_be_bytes());
        output.extend_from_slice(payload);
        output.extend_from_slice(&jpeg[2..]);
        output
    }
}
