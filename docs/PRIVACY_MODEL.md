# Privacy Model

## Local Data

900Word uses user-selected document paths for saved files.

Runtime recent-document state keeps actual paths backend-only. The frontend receives opaque tokens and generic labels such as `Recent document 1`, not local paths or private filenames.

Autosave writes local recovery snapshots with versioned opaque tokens. On Unix platforms the recovery directory is forced to owner-only permissions and recovery files are written with owner-only permissions. Recovery summaries exposed to the frontend include only the validated opaque token, a generic label, byte length, and modified timestamp. Sprint 029 bounds retention to 3 snapshots per document and 20 snapshots overall. Existing single-file recovery tokens remain accepted for list, recover, and discard, but new autosaves write only the versioned snapshot shape. Recovery files are not encrypted and may contain document content, so encryption remains a deferred feature.

The desktop shell uses native Tauri dialogs for ODT open and Save As flows. Dialog output is still treated as untrusted input: Rust commands validate extensions, traversal components, and document size limits before read/write operations. The frontend does not display private local paths after selection.

Spell-check user dictionaries live under `{APP_DATA_DIR}/dictionaries`. The backend creates this folder with owner-only permissions on Unix platforms. Frontend dictionary state includes language tag, display name, source type, and license label only; it does not include local dictionary paths or filenames.

TXT, HTML, and PDF export paths are user-entered in the desktop shell. Backend export commands validate format-specific extensions and traversal components, write atomically, and return only export format and byte length to frontend state. PDF page-range options are validated with generic errors. Export success and validation messages do not include private filenames, local paths, or document text.

Sprint 011 hyperlink editing stores user-entered safe link targets inside document content. The editor validates links locally and does not fetch, preview, or open targets while editing.

Sprint 012 table editing stores supported cell content in the existing local document model. Unsupported cell content keeps the editor projection read-only rather than serializing private details into UI state or logs.

Sprint 013 header/footer editing stores simple page-region paragraphs and typed page fields in the existing local document model. Unsupported imported header/footer complexity is marked read-only with generic warnings. Page-number, page-count, and date field rendering does not inspect local paths, account data, hostnames, or network state.

Sprint 014 image insertion reads a user-selected local image path only inside the Rust IPC command. The source path is not stored after validation. Accepted assets receive generated `image-<UUID>.<ext>` identifiers and `original_name` is left empty for local imports so private source filenames are not serialized. Frontend document state may contain embedded asset bytes for projection, but not source paths or private filenames.

Sprint 022 comments are stored as bounded local document metadata plus inline selected-text anchors. The default author string is `Local User`; no operating-system username, hostname, account identifier, contact record, local path, or source filename is read or serialized for comment authorship.

Sprint 023 tracked changes are stored as local document metadata on inline text anchors. The default author string is `Local User`; no operating-system username, hostname, account identifier, contact record, local path, source filename, or cloud identity is read or serialized for tracked-change authorship. Unaccepted tracked deletions intentionally keep deleted text in the saved document package, so shared files may reveal edit history until changes are accepted or rejected.

Sprint 024 table-of-contents entries are stored as generated local document content. Entry text is copied from supported document headings, and generated bookmark IDs are compact document-local identifiers. The feature does not read operating-system usernames, hostnames, account identifiers, contact records, local paths, source filenames, cloud identity, network state, or external services.

Sprint 025 footnotes and endnotes are stored as bounded local document content. Note IDs are compact generated document-local identifiers, labels are bounded visible reference strings, and bodies are bounded text. Stored note bodies appear in the desktop Notes sidebar when local notes are present, including notes promoted from bounded safe ODT metadata. The feature does not read operating-system usernames, hostnames, account identifiers, contact records, local paths, source filenames, cloud identity, network state, or external services.

Sprint 026 smart typing settings are local UI settings and are not written into document metadata. The typed-input transforms operate on the active editor text around the cursor only and use a small bundled typo replacement map. The feature does not read operating-system usernames, hostnames, account identifiers, contact records, local paths, source filenames, cloud identity, network state, or external services.

Sprint 027 document statistics are derived in the desktop UI from existing document state, backend count summaries, and the existing editor selection snapshot. The frontend panel shows aggregate counts, estimates, and privacy-relevant local document indicators only. It does not receive or render private local paths, private filenames, operating-system usernames, hostnames, account identifiers, contacts, cloud identity, source image filenames, recovery locations, network state, or external services.

Sprint 028 accessibility and low-resource settings are local desktop UI preferences. They control toolbar sizing, app-controlled motion, and nonessential automatic UI surfaces only. They are not written into document content, saved packages, exports, document metadata, logs, or remote services.

Sprint 030 document inspector summaries are derived in the desktop UI from existing local document state, backend count summaries, and generic file-session state. The frontend inspector shows aggregate format, saved/unsaved, document-created and document-modified metadata timestamps, page, stats, embedded-image-byte, review, note, and privacy-warning indicators only. Saved paths remain backend-only, and the inspector does not render private local paths, private filenames, source image filenames, operating-system usernames, hostnames, account identifiers, contacts, cloud identity, recovery locations, network state, external services, or document text beyond already visible editor content.

Sprint 032 PDF export settings are local UI state only. Page-range start/end values are sent to Rust as typed options and are not stored in the document. Invalid or empty ranges return generic errors without local paths, private filenames, usernames, hostnames, or document text.

Sprint 033 DOCX page-region import/export uses only local package relationships and preflighted header/footer XML parts. Unsafe, remote, missing, or unsupported page-region relationships produce generic warnings without exposing package entry names, local paths, private filenames, usernames, hostnames, or document text. Exported DOCX page-region parts are generated from `word-core` and do not include source paths, account data, telemetry identifiers, or remote references.

Sprint 034 DOCX image import/export uses only local package image relationships and preflighted embedded media parts. Accepted imported images receive generated `docx-image-<n>.<ext>` asset IDs, leave `original_name` empty, and do not preserve source relationship target names or private filenames. Unsafe, remote, missing, mismatched, unsupported, or over-limit image media produce generic warnings without package entry names, local paths, private filenames, usernames, hostnames, or document text. Exported DOCX image parts are generated from `word-core` asset bytes and do not include source paths, account data, telemetry identifiers, linked image references, or remote references.

Sprint 035 DOCX comments import/export uses only local package comments relationships and preflighted `word/comments.xml` or `word/comments*.xml` parts. Accepted imported comments receive generated local `word-core` comment IDs and attach only to supported visible inline text ranges. Unsafe, remote, missing, malformed, duplicate, unanchored, threaded/reply, unsupported, or over-limit comments produce generic warnings without package entry names, relationship targets, local paths, private filenames, usernames, hostnames, or comment body text. Exported DOCX comments use generated numeric DOCX IDs, a generated `word/comments.xml` part, and generated relationship metadata; they do not include source paths, account data, telemetry identifiers, remote references, OS usernames, or hostnames. DOCX resolved-state fidelity remains deferred.

## Logs

Logs may include high-level operation names and error categories. Logs must not include document text, private filenames, local paths, or recovered content.

## Metadata

Exporters must avoid adding local usernames, hostnames, absolute paths, or private build metadata to ODT, HTML, TXT, PDF, or EPUB outputs.

Sprint 007 HTML and print exports are generated from the `word-core` model with offline CSP metadata and no remote image or script emission. The PDF adapter is generated locally and does not embed local path metadata.

Sprint 013 TXT, HTML, and print HTML exports include simple header/footer text and render page fields with predictable placeholder values where pagination is not available. Sprint 032 PDF export renders simple header/footer text and page fields from generated page numbers, total page count, and document modified date values. These outputs do not add local usernames, hostnames, absolute paths, or private build metadata.

Sprint 014 HTML and print HTML exporters embed allowlisted in-document image bytes as `data:` URLs. They do not emit remote image URLs, `file:` URLs, source paths, original local filenames, usernames, hostnames, or private build metadata. The PDF exporter remains text-oriented and may include image alt/caption text only.

Sprint 022 stores 900Word-authored comments in ODT with ODF annotation elements and `word900` metadata for local comment ID and resolved state. TXT, HTML, print HTML, and PDF export do not claim comment fidelity or active annotations; they continue to avoid local usernames, hostnames, absolute paths, account metadata, and private build metadata.

Sprint 023 stores 900Word-authored tracked changes in ODT with `word900` metadata on inline text spans for local change ID, kind, author, and timestamp, plus document-level recording state. This is a 900Word-authored text-only compatibility boundary, not a claim of DOCX/PDF track changes or full external ODT change-tracking fidelity.

Sprint 024 stores 900Word-authored table-of-contents metadata in ODT with `word900:block-type="table-of-contents"` and safe generated bookmark targets. TXT/PDF exports render TOCs as ordinary text, and HTML/print HTML exports render safe local fragment links without deterministic TOC page-number claims or local/private build metadata.

Sprint 025 stores 900Word-authored footnotes and endnotes in ODT with ODF note elements and bounded `word900` metadata for local note ID and kind. Notes imported as local note metadata are visible in the desktop Notes sidebar, while malformed or unsupported note structures fall back to ordinary visible text with generic warnings. TXT/PDF exports render notes as ordinary text, and HTML/print HTML exports render sanitized local note sections without deterministic page-bottom placement, active PDF annotations, local usernames, hostnames, absolute paths, account metadata, or private build metadata.

Sprint 026 does not add ODT, TXT, HTML, print HTML, or PDF metadata. Text produced by smart typing is ordinary user-authored document text after the user types it.

Sprint 027 does not add ODT, TXT, HTML, print HTML, PDF, or app metadata. The expanded stats panel is an ephemeral desktop UI projection and does not change saved document packages or exported files.

Sprint 028 does not add ODT, TXT, HTML, print HTML, PDF, or app metadata. The accessibility and low-resource controls remain desktop UI settings and do not change saved document packages or exported files.

Sprint 030 does not add ODT, TXT, HTML, print HTML, PDF, or app metadata. The Document Inspector is an ephemeral desktop UI projection and does not change saved document packages or exported files.

Sprint 032 PDF export does not add local path metadata, source filenames, usernames, hostnames, creation-date metadata, producer metadata, telemetry identifiers, remote resources, or private build metadata to generated PDFs.

Sprint 033 DOCX export adds simple generated header/footer XML parts only when `word-core` page regions have content. These generated parts do not include local path metadata, source filenames, usernames, hostnames, account identifiers, telemetry identifiers, remote resources, custom XML, macros, embedded objects, or private build metadata.

Sprint 034 DOCX export embeds only valid allowlisted image assets already present in the local document model. Exported image relationship targets and media part names are generated and do not use source filenames, local paths, usernames, hostnames, account identifiers, telemetry identifiers, remote resources, custom XML, macros, embedded objects, or private build metadata.

Sprint 035 DOCX export writes only simple anchored comments already present in the local document model. Exported comment part names, relationship IDs, and numeric comment IDs are generated and do not use source filenames, local paths, usernames, hostnames, account identifiers, telemetry identifiers, remote resources, custom XML, macros, embedded objects, or private build metadata.

## Network

Core editing workflows must run offline. Any future network feature must be opt-in and documented in an ADR.

Spell-check dictionaries are local only in the Sprint 006 implementation. Remote dictionary downloads and asset stores remain deferred.
