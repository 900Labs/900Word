# Threat Model

## Protected Assets

- User document contents.
- Local filesystem outside selected document paths.
- Autosave and recovery data.
- Exported document metadata.
- Application integrity.

## Trust Boundaries

- Imported ODT, DOCX, HTML, TXT, images, templates, fonts, and dictionaries.
- Tauri IPC between the frontend and Rust backend.
- File dialogs and user-selected paths.
- Future plugins, sync services, and downloadable assets.

## Current Mitigations

- Rust validates IPC inputs.
- Frontend receives sanitized projections.
- Core app has no shell plugin.
- Public-release scanner blocks local paths, secrets, and generated artifacts.

## Deferred Threat Models

- Runtime plugins.
- Cloud sync and collaboration.
- Full encryption.
- External document converters.
