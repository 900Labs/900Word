# Privacy

900Word is local-first and has no telemetry by default.

Documents stay on the user's machine unless the user explicitly exports or shares them. The bootstrap application does not include cloud sync, real-time collaboration, remote dictionaries, or plugin marketplaces.

## Privacy Rules

- Do not log document text.
- Do not log private filenames or full local paths.
- Do not send network requests from core editing workflows.
- Do not commit real user documents as fixtures.
- Do not expose recent-document paths or recovery-file paths to frontend state.
- Do not claim encryption until recovery, temp-file, metadata, and key-handling behavior are implemented and tested.

## Bootstrap Enforcement

The desktop shell keeps telemetry off even when settings input attempts to enable it. The workspace tests also scan startup frontend sources for browser network primitives so the initial app shell remains offline by default.

Sprint 004 backend tests verify that recent-document summaries use generic labels and opaque tokens instead of private paths or filenames, and that recovery tokens reject traversal or path-shaped input.

Sprint 029 replaces single-file recovery overwrites with bounded versioned recovery snapshots. Snapshot tokens are validated opaque values, recovery labels stay generic, recovered drafts open dirty and unsaved, and discard only accepts the selected validated token. Retention is capped at 3 snapshots per document and 20 snapshots overall. The feature does not expose local paths, filenames, usernames, hostnames, source document names, recovery locations, telemetry, network calls, cloud sync, accounts, AI services, document-content logging, encryption claims, import/export changes, or new document metadata.

Sprint 006 keeps dictionary discovery local. The app creates `{APP_DATA_DIR}/dictionaries` for user-provided Hunspell `.aff`/`.dic` pairs and does not expose that local path to frontend state, logs, docs examples, or release artifacts.

Sprint 042 adds a Settings dictionary manager on top of the existing local dictionary commands. It shows bundled/user source type, license, language tag, display name, and generic source labels only. It does not expose local paths, private filenames, dictionary filenames, usernames, hostnames, source document names, telemetry identifiers, cloud identity, or network state, and it does not add downloads, remote dictionary lookup, telemetry, cloud sync, accounts, plugin runtime behavior, or document-content logging.

Sprint 043 persists sanitized app settings in the existing app-data boundary. Stored settings include only UI/preferences fields such as language tag, UI locale, contrast, toolbar sizing, reduced motion, low-resource mode, and smart typing toggles. Telemetry is forced off before save and after load. The settings file does not store document paths, recent paths, private filenames, document text, usernames, hostnames, account/cloud identity, telemetry identifiers, network state, or plugin/runtime state. Missing, malformed, unreadable, oversized, symlinked, or path-shaped settings fall back to sanitized defaults or generic save errors without exposing the app-data path.

Sprint 044 adds a local Settings reset. Reset rewrites the existing local settings file with sanitized defaults through the same private atomic settings path instead of deleting it. The returned defaults keep telemetry disabled and do not expose local paths, filenames, usernames, hostnames, account/cloud identity, telemetry identifiers, network state, document text, recent paths, or recovery locations. Reset does not add settings import/export, cloud sync, accounts, telemetry, network behavior, plugin-managed settings, document metadata, recent-document persistence, or recovery behavior changes.

Sprint 007 keeps exports local. TXT, HTML, and PDF export commands write only to user-entered paths after backend extension and traversal validation. Export status exposed to the frontend includes only the export format and byte length, not private filenames or local paths.

Sprint 031 adds DOCX import/export conversion while keeping ODT canonical. Opening a `.docx` validates the path and package in Rust, imports supported content into an unsaved dirty document, and does not expose the source path or filename to frontend document state. DOCX export writes only to a user-entered `.docx` path after backend extension and traversal validation. Status exposed to the frontend includes only format and byte length. Sprint 034 imports only safe embedded image payloads under generated asset IDs and does not preserve DOCX relationship target names or source filenames. Sprint 035 imports only simple anchored comments from safe local comments parts, generates local comment IDs, and keeps warnings generic without package entry names, relationship targets, local paths, private filenames, usernames, hostnames, or comment body text. Sprint 039 imports only simple text-only DOCX insertion/deletion revisions into generated local tracked-change IDs. Bounded safe DOCX revision authors may be preserved, but path-like, account-like, hostname-like, missing, or over-limit author metadata falls back to generic local-safe labels; malformed dates fall back to a deterministic epoch timestamp. DOCX revision export uses generated numeric IDs and sanitized local tracked-change author/date metadata, never source filenames, paths, usernames, hostnames, account metadata, telemetry IDs, or private build metadata. Sprint 040 imports only simple DOCX footnotes/endnotes from safe local note parts into generated local note IDs and generic labels, keeps unsupported-note warnings free of package paths, raw private IDs, filenames, usernames, hostnames, and hidden note body text, and exports generated numeric DOCX note IDs only. The converter does not fetch remote relationships, import linked or remote media payloads, run macros, invoke external converters, add telemetry, read usernames or hostnames, create accounts, or send document content to network services.

Sprint 009 enables native WebView spellcheck attributes as a low-cost editor hint while keeping the explicit Hunspell check workflow local. 900Word does not add remote dictionary lookup, telemetry, or document-text upload paths.

Sprint 010 adds explicit Hunspell-backed red underlines, bounded local suggestions, ignore-once/all session actions, and local personal dictionary additions. Personal dictionary words are stored in app data and are not sent to remote services.

Sprint 011 adds a heading navigator and hyperlink editor. The navigator is derived from the in-memory document model. The link editor validates `http`, `https`, and `mailto` targets locally and does not open, prefetch, or contact link targets during editing.

Sprint 012 adds editable table projection for supported local document content. Table editing does not add telemetry, cloud sync, remote resource fetching, or document-content logging.

Sprint 013 adds local header/footer editing and typed page fields. Page field values are generated from the document model and exporter context only; they do not read usernames, hostnames, absolute paths, accounts, network state, or external services.

Sprint 014 adds local image insertion. Image files selected through the native dialog are treated as untrusted local input and validated in Rust for traversal, extension, type, byte size, and magic bytes. Accepted bytes are copied into embedded document assets under generated generic asset names. Source local paths and source filenames are not stored in frontend state, document assets, exports, docs, logs, or fixtures. HTML export emits only embedded `data:` URLs for allowlisted in-document assets and never remote or `file:` image URLs.

Sprint 015 adds local editing for image alt text, captions, alignment, and scale. These values are stored as document content/presentation metadata only. They do not include source local paths or private source filenames, and editing them does not add telemetry, cloud sync, remote image fetching, or document-content logging.

Sprint 038 adds PDF embedding for safe JPEG assets already stored inside the local document model. PDF export does not read external image paths, fetch remote image URLs, preserve source filenames, emit asset IDs, or add PDF creation/producer metadata. APP/COM JPEG metadata marker segments are stripped before embedding. Unsupported, malformed, metadata-after-scan, or over-limit images remain visible placeholders with document-authored alt/caption text. EXIF interpretation and selective metadata preservation remain deferred.

Sprint 016 adds local bookmarks and internal links. Bookmark IDs are generated compact document identifiers stored on supported paragraph/heading blocks. Internal link target lists are derived from the in-memory document model and do not contact link targets, resolve local paths, store accounts, store source filenames, or add telemetry/cloud sync.

Sprint 018 adds direct selected-image resizing by updating existing bounded image scale metadata. Oversized image imports surface generic compress-or-resize guidance and do not expose source paths, source filenames, usernames, or filesystem details. Image bytes remain embedded document assets; no remote loading, telemetry, accounts, cloud sync, or document-content logging is added.

Sprint 019 expands the generated starter template gallery. Templates are built from placeholder-only `word-core` blocks and do not embed real user documents, local paths, source filenames, organization names, private endpoints, disk images, remote assets, telemetry, accounts, cloud sync, or document-content logging.

Sprint 022 adds local comments on selected text. Comment authors default to `Local User`; the feature does not read usernames, hostnames, accounts, local paths, source filenames, contacts, or cloud identity. Comment bodies and IDs are bounded and stored only in the local document model and saved document package. Comments add no telemetry, cloud sync, remote lookup, account system, or document-content logging.

Sprint 023 adds local text-only tracked changes. Tracked-change authors default to `Local User`; the feature does not read usernames, hostnames, account identifiers, contacts, local paths, source filenames, or cloud identity. Tracked changes can reveal edit history and deleted text in saved documents until accepted or rejected, so users should review and resolve changes before sharing files when that history is sensitive.

Sprint 024 adds local table-of-contents generation from supported document headings. TOC entries and generated bookmark IDs are derived only from the in-memory document model. The feature does not read usernames, hostnames, account identifiers, contacts, local paths, source filenames, cloud identity, network state, or external services, and it adds no telemetry, cloud sync, or remote lookup.

Sprint 025 adds local footnotes and endnotes. Note IDs, labels, kinds, and bodies are bounded document content stored only in the local document model and saved document package. Stored note bodies are surfaced in the desktop Notes sidebar so imported local notes are inspectable rather than hidden behind inline markers. Note insertion does not read usernames, hostnames, account identifiers, contacts, local paths, source filenames, cloud identity, network state, or external services, and it adds no telemetry, cloud sync, remote lookup, or document-content logging.

Sprint 026 adds local smart typing and autocorrect behavior for user-typed editor input only. The feature uses disabled-by-default local settings and a small bundled typo replacement map; it does not inspect imported documents for cleanup, send document text to remote services, add accounts, add telemetry, read usernames, read hostnames, read local paths, or store new document metadata.

Sprint 027 expands the local Stats panel using the in-memory document model and existing editor selection snapshot. The panel shows counts and estimates only; it does not display local paths, private filenames, usernames, hostnames, accounts, source image filenames, or recovery locations. It adds no telemetry, network calls, cloud sync, accounts, AI services, document-content logging, import/export changes, or new document metadata.

Sprint 028 adds local accessibility and low-resource settings for larger toolbar controls, reduced motion, and low-resource mode. These settings affect only the desktop UI shell, default off, and are not written into documents, exports, document metadata, or logs. They add no telemetry, network calls, cloud sync, accounts, AI services, document-content logging, import/export changes, or document metadata.
