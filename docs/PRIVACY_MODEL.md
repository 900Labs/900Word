# Privacy Model

## Local Data

900Word uses user-selected document paths for saved files.

Runtime recent-document state keeps actual paths backend-only. The frontend receives opaque tokens and generic labels such as `Recent document 1`, not local paths or private filenames.

Autosave writes recovery drafts under `{TEMP_DIR}/900word-recovery` using sanitized `recovery-<DOCUMENT_ID>.odt` filenames. On Unix platforms the recovery directory is forced to owner-only permissions and recovery files are written with owner-only permissions. Recovery summaries exposed to the frontend include only an opaque token, generic label, byte length, and modified timestamp. Recovery files are not encrypted in the Sprint 004 implementation and may contain document content, so encryption remains a deferred feature.

The Sprint 004 desktop shell uses explicit path entry instead of a native file picker. Rust commands validate extensions, traversal components, and document size limits, but native picker-granted file scopes remain deferred until dialog permissions are added and reviewed.

## Logs

Logs may include high-level operation names and error categories. Logs must not include document text, private filenames, local paths, or recovered content.

## Metadata

Exporters must avoid adding local usernames, hostnames, absolute paths, or private build metadata to ODT, HTML, TXT, PDF, or EPUB outputs.

## Network

Core editing workflows must run offline. Any future network feature must be opt-in and documented in an ADR.
