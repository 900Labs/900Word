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

Sprint 006 keeps dictionary discovery local. The app creates `{APP_DATA_DIR}/dictionaries` for user-provided Hunspell `.aff`/`.dic` pairs and does not expose that local path to frontend state, logs, docs examples, or release artifacts.

Sprint 007 keeps exports local. TXT, HTML, and PDF export commands write only to user-entered paths after backend extension and traversal validation. Export status exposed to the frontend includes only the export format and byte length, not private filenames or local paths.
