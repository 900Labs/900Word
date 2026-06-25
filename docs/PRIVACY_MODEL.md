# Privacy Model

## Local Data

900Word uses user-selected document paths for saved files.

Runtime recent-document state keeps actual paths backend-only. The frontend receives opaque tokens and generic labels such as `Recent document 1`, not local paths or private filenames.

Autosave writes recovery drafts under `{TEMP_DIR}/900word-recovery` using sanitized `recovery-<DOCUMENT_ID>.odt` filenames. On Unix platforms the recovery directory is forced to owner-only permissions and recovery files are written with owner-only permissions. Recovery summaries exposed to the frontend include only an opaque token, generic label, byte length, and modified timestamp. Recovery files are not encrypted in the Sprint 004 implementation and may contain document content, so encryption remains a deferred feature.

The desktop shell uses native Tauri dialogs for ODT open and Save As flows. Dialog output is still treated as untrusted input: Rust commands validate extensions, traversal components, and document size limits before read/write operations. The frontend does not display private local paths after selection.

Spell-check user dictionaries live under `{APP_DATA_DIR}/dictionaries`. The backend creates this folder with owner-only permissions on Unix platforms. Frontend dictionary state includes language tag, display name, source type, and license label only; it does not include local dictionary paths or filenames.

TXT, HTML, and PDF export paths are user-entered in the Sprint 007 shell. Backend export commands validate format-specific extensions and traversal components, write atomically, and return only export format and byte length to frontend state. Export success messages do not include private filenames or local paths.

Sprint 011 hyperlink editing stores user-entered safe link targets inside document content. The editor validates links locally and does not fetch, preview, or open targets while editing.

Sprint 012 table editing stores supported cell content in the existing local document model. Unsupported cell content keeps the editor projection read-only rather than serializing private details into UI state or logs.

Sprint 013 header/footer editing stores simple page-region paragraphs and typed page fields in the existing local document model. Unsupported imported header/footer complexity is marked read-only with generic warnings. Page-number, page-count, and date field rendering does not inspect local paths, account data, hostnames, or network state.

Sprint 014 image insertion reads a user-selected local image path only inside the Rust IPC command. The source path is not stored after validation. Accepted assets receive generated `image-<UUID>.<ext>` identifiers and `original_name` is left empty for local imports so private source filenames are not serialized. Frontend document state may contain embedded asset bytes for projection, but not source paths or private filenames.

Sprint 022 comments are stored as bounded local document metadata plus inline selected-text anchors. The default author string is `Local User`; no operating-system username, hostname, account identifier, contact record, local path, or source filename is read or serialized for comment authorship.

## Logs

Logs may include high-level operation names and error categories. Logs must not include document text, private filenames, local paths, or recovered content.

## Metadata

Exporters must avoid adding local usernames, hostnames, absolute paths, or private build metadata to ODT, HTML, TXT, PDF, or EPUB outputs.

Sprint 007 HTML and print exports are generated from the `word-core` model with offline CSP metadata and no remote image or script emission. The basic PDF adapter is generated locally and does not embed local path metadata.

Sprint 013 TXT, HTML, print HTML, and basic PDF exports include simple header/footer text and render page fields with predictable placeholder values where pagination is not available. These outputs do not add local usernames, hostnames, absolute paths, or private build metadata.

Sprint 014 HTML and print HTML exporters embed allowlisted in-document image bytes as `data:` URLs. They do not emit remote image URLs, `file:` URLs, source paths, original local filenames, usernames, hostnames, or private build metadata. The basic PDF exporter remains text-oriented and may include image alt/caption text only.

Sprint 022 stores 900Word-authored comments in ODT with ODF annotation elements and `word900` metadata for local comment ID and resolved state. TXT, HTML, print HTML, and basic PDF export do not claim comment fidelity or active annotations; they continue to avoid local usernames, hostnames, absolute paths, account metadata, and private build metadata.

## Network

Core editing workflows must run offline. Any future network feature must be opt-in and documented in an ADR.

Spell-check dictionaries are local only in the Sprint 006 implementation. Remote dictionary downloads and asset stores remain deferred.
