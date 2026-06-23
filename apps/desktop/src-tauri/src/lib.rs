use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use word_core::{Document, DocumentCommand, DocumentError, DocumentStats, UndoStack};
use word_spell::{DictionaryInfo, SpellIssue};

const MAX_DOCUMENT_BYTES: u64 = 32 * 1024 * 1024;
const MAX_RECENT_DOCUMENTS: usize = 5;
const RECOVERY_DIR_NAME: &str = "900word-recovery";

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
    pub telemetry_enabled: bool,
    pub language_tag: String,
    pub high_contrast: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentFileState {
    pub has_current_path: bool,
    pub dirty: bool,
    pub recent_documents: Vec<RecentDocumentSummary>,
    pub recovery_documents: Vec<RecoveryDocumentSummary>,
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
    session.dirty = false;
    Ok(session.document.clone())
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
    let path = recovery_path_for_token(&token)?;
    let document = read_document_from_path(&path)?;
    let mut session = lock_session(&state)?;
    session.document = document;
    session.undo = UndoStack::default();
    session.current_path = None;
    session.dirty = true;
    Ok(session.document.clone())
}

#[tauri::command]
fn discard_recovery(token: String) -> Result<(), String> {
    let path = recovery_path_for_token(&token)?;
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
    let token = recovery_token_for_document(document);
    let path = recovery_path_for_token(&token)?;
    let bytes = word_odf::write_odt_bytes(document).map_err(|err| err.to_string())?;
    ensure_private_recovery_dir()?;
    write_bytes_atomically(&path, &bytes, true)?;
    recovery_summary_from_path(token, &path, 1)
}

fn list_recovery_documents_from_disk() -> Result<Vec<RecoveryDocumentSummary>, String> {
    let dir = recovery_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut tokens = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(safe_io_error)? {
        let entry = entry.map_err(safe_io_error)?;
        let Some(token) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };
        if validate_recovery_token(&token).is_ok() {
            tokens.push(token);
        }
    }
    tokens.sort();

    let mut summaries = Vec::new();
    for (index, token) in tokens.into_iter().enumerate() {
        let path = recovery_path_for_token(&token)?;
        summaries.push(recovery_summary_from_path(token, &path, index + 1)?);
    }
    Ok(summaries)
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
    format!("recovery-{}.odt", document.id)
}

fn recovery_path_for_token(token: &str) -> Result<PathBuf, String> {
    let token = validate_recovery_token(token)?;
    Ok(recovery_dir().join(token))
}

fn recovery_dir() -> PathBuf {
    std::env::temp_dir().join(RECOVERY_DIR_NAME)
}

fn ensure_private_recovery_dir() -> Result<PathBuf, String> {
    let dir = recovery_dir();
    std::fs::create_dir_all(&dir).map_err(safe_io_error)?;
    set_private_directory_permissions(&dir)?;
    Ok(dir)
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
    if token.len() > 64
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
    Ok(token.to_string())
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
    fn recovery_tokens_reject_traversal_and_plain_paths() {
        assert!(
            validate_recovery_token("recovery-00000000-0000-4000-8000-000000000001.odt").is_ok()
        );
        assert_eq!(
            validate_recovery_token("../private.odt").expect_err("traversal should fail"),
            "recovery token is invalid"
        );
        assert_eq!(
            validate_recovery_token("folder/recovery-private.odt").expect_err("path should fail"),
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
        let settings = update_settings(Settings {
            telemetry_enabled: true,
            language_tag: "en".to_string(),
            high_contrast: true,
        });

        assert!(!settings.telemetry_enabled);
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
    fn default_capability_keeps_shell_access_out_of_core() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let capability = std::fs::read_to_string(manifest_dir.join("capabilities/default.json"))
            .expect("default capability must be readable");

        assert!(capability.contains("\"core:default\""));
        assert!(!capability.contains("shell"));
        assert!(!capability.contains("http:"));
        assert!(!capability.contains("https:"));
    }
}
