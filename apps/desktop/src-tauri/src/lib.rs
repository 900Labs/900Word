use serde::{Deserialize, Serialize};
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
use word_spell::{DictionaryInfo, SpellIssue};

const MAX_DOCUMENT_BYTES: u64 = 32 * 1024 * 1024;
const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;
const IMAGE_TOO_LARGE_ERROR: &str = "image file is too large";
const MAX_RECENT_DOCUMENTS: usize = 5;
const RECOVERY_DIR_NAME: &str = "900word-recovery";
const USER_DICTIONARY_DIR_NAME: &str = "dictionaries";
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
    pub telemetry_enabled: bool,
    pub language_tag: String,
    pub ui_locale: String,
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

impl Default for Settings {
    fn default() -> Self {
        Self {
            telemetry_enabled: false,
            language_tag: FALLBACK_LANGUAGE_TAG.to_string(),
            ui_locale: "en-US".to_string(),
            high_contrast: false,
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
            prepare_print_html,
            check_spelling,
            add_to_personal_dictionary,
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
fn export_pdf(state: State<'_, AppState>) -> Result<Vec<u8>, String> {
    let session = lock_session(&state)?;
    word_export::export_basic_pdf(&session.document).map_err(|err| err.to_string())
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
    state: State<'_, AppState>,
) -> Result<ExportFileResult, String> {
    let path = validate_path(&path, "pdf")?;
    let session = lock_session(&state)?;
    let pdf = word_export::export_basic_pdf(&session.document).map_err(|err| err.to_string())?;
    write_export_bytes_to_path("pdf", &path, &pdf)
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
fn list_dictionaries(app: tauri::AppHandle) -> Result<Vec<DictionaryInfo>, String> {
    let user_root = user_dictionary_dir(&app)?;
    ensure_user_dictionary_dir(&user_root)?;
    Ok(word_spell::list_dictionaries_with_user_root(&user_root))
}

#[tauri::command]
fn get_settings() -> Settings {
    Settings::default()
}

#[tauri::command]
fn update_settings(settings: Settings) -> Settings {
    Settings {
        telemetry_enabled: false,
        language_tag: normalize_language_setting(&settings.language_tag),
        ui_locale: normalize_ui_locale(&settings.ui_locale),
        high_contrast: settings.high_contrast,
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

fn normalize_language_setting(language_tag: &str) -> String {
    if language_tag.trim().is_empty() {
        FALLBACK_LANGUAGE_TAG.to_string()
    } else {
        language_tag.replace('_', "-")
    }
}

fn normalize_ui_locale(ui_locale: &str) -> String {
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

    let bytes = fs::read(path).map_err(safe_io_error)?;
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

    Ok((extension, detected_media_type, bytes))
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
            ui_locale: "unknown".to_string(),
            high_contrast: true,
        });

        assert!(!settings.telemetry_enabled);
        assert_eq!(settings.language_tag, "en");
        assert_eq!(settings.ui_locale, "en-US");
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
        assert!(privacy_model.contains("The default author string is `Local User`"));
        assert!(privacy_model.contains("no operating-system username"));
        assert!(file_formats.contains("900Word-authored text-only tracked changes"));
        assert!(file_formats.contains("`word900` metadata"));
        assert!(file_formats.contains("DOCX track changes"));
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
}
