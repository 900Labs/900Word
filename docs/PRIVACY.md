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
