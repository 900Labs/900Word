use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use tauri::State;
use word_core::{Document, DocumentCommand, DocumentError, DocumentStats, UndoStack};
use word_spell::{DictionaryInfo, SpellIssue};

const MAX_DOCUMENT_BYTES: u64 = 32 * 1024 * 1024;

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
}

impl Default for DocumentSession {
    fn default() -> Self {
        Self {
            document: Document::new_untitled(),
            undo: UndoStack::default(),
            current_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub telemetry_enabled: bool,
    pub language_tag: String,
    pub high_contrast: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            telemetry_enabled: false,
            language_tag: "en".to_string(),
            high_contrast: false,
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            new_document,
            open_document,
            save_document,
            save_document_as,
            get_document_state,
            apply_document_command,
            undo,
            redo,
            get_document_stats,
            export_txt,
            export_html,
            export_pdf,
            check_spelling,
            list_dictionaries,
            get_settings,
            update_settings,
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
    Ok(session.document.clone())
}

#[tauri::command]
fn open_document(path: String, state: State<'_, AppState>) -> Result<Document, String> {
    let path = validate_path(&path, "odt")?;
    let metadata = std::fs::metadata(&path).map_err(safe_io_error)?;
    if metadata.len() > MAX_DOCUMENT_BYTES {
        return Err("document exceeds supported bootstrap size limit".to_string());
    }
    let bytes = std::fs::read(&path).map_err(safe_io_error)?;
    let document = word_odf::read_odt_bytes(&bytes).map_err(|err| err.to_string())?;

    let mut session = lock_session(&state)?;
    session.document = document;
    session.undo = UndoStack::default();
    session.current_path = Some(path);
    Ok(session.document.clone())
}

#[tauri::command]
fn save_document(state: State<'_, AppState>) -> Result<(), String> {
    let session = lock_session(&state)?;
    let path = session
        .current_path
        .clone()
        .ok_or_else(|| "no current document path; use save_document_as".to_string())?;
    write_document_to_path(&session.document, &path)
}

#[tauri::command]
fn save_document_as(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let path = validate_path(&path, "odt")?;
    let mut session = lock_session(&state)?;
    write_document_to_path(&session.document, &path)?;
    session.current_path = Some(path);
    Ok(())
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
    Ok(session.document.clone())
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
fn export_pdf(state: State<'_, AppState>) -> Result<Vec<u8>, String> {
    let session = lock_session(&state)?;
    word_export::export_basic_pdf(&session.document).map_err(|err| err.to_string())
}

#[tauri::command]
fn check_spelling(text: String, language_tag: String) -> Result<Vec<SpellIssue>, String> {
    let checker = word_spell::checker_for(&language_tag).map_err(|err| err.to_string())?;
    Ok(checker.check(&text))
}

#[tauri::command]
fn list_dictionaries() -> Vec<DictionaryInfo> {
    word_spell::list_dictionaries()
}

#[tauri::command]
fn get_settings() -> Settings {
    Settings::default()
}

#[tauri::command]
fn update_settings(settings: Settings) -> Settings {
    Settings {
        telemetry_enabled: false,
        ..settings
    }
}

fn write_document_to_path(document: &Document, path: &Path) -> Result<(), String> {
    let bytes = word_odf::write_odt_bytes(document).map_err(|err| err.to_string())?;
    std::fs::write(path, bytes).map_err(safe_io_error)
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

    #[test]
    fn rejects_wrong_extension() {
        let err = validate_path("document.txt", "odt").expect_err("txt path should fail");

        assert_eq!(err, "expected .odt document path");
    }

    #[test]
    fn rejects_parent_traversal() {
        let err = validate_path("../document.odt", "odt").expect_err("traversal should fail");

        assert_eq!(err, "path contains unsupported traversal");
    }

    #[test]
    fn settings_never_enable_telemetry() {
        let settings = update_settings(Settings {
            telemetry_enabled: true,
            language_tag: "en".to_string(),
            high_contrast: true,
        });

        assert!(!settings.telemetry_enabled);
    }
}
