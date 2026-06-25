# Sprint 051: User Template Library MVP

## Status

Complete.

## Goal

Add a bounded offline user template library while keeping ODT canonical, `word-core` as the source of truth, generated template IDs stable, and template summaries/errors free of private local details.

## Shipped

- Added app-data-scoped user template storage under a local template folder with owner-only directory and file permissions on Unix platforms where supported.
- Stored user templates as ODT bytes plus minimal JSON metadata containing only a generated opaque template ID and sanitized display name.
- Added backend commands to save the current document as a user template, list generated and user templates together, load a user template as a fresh unsaved document, and delete only user templates.
- Kept generated templates code-built, stable, and undeletable.
- Added compact desktop toolbar controls for template name entry, save current document as template, use selected template, delete selected user template, and generic source/description display.
- Added English and Spanish UI strings while preserving English fallback behavior.

## Compatibility Boundary

- ODT remains the canonical saved document and template byte format.
- Loading a user template creates a cloned `word-core` document with a new document ID, `current_path = None`, and clean unsaved session state like generated template loading.
- Template IDs for user templates are generated opaque UUID tokens, not display names, filenames, or paths.
- Template summaries expose only ID, sanitized display name, generic source, generic description, and deletion capability.

## Privacy And Safety

- User template ODT files contain the document content the user intentionally saved as reusable template material.
- List summaries and errors do not expose local paths, source filenames, usernames, hostnames, account/cloud identifiers, telemetry identifiers, or document text.
- Path-shaped names collapse to a generic display name.
- Traversal-shaped IDs, plain paths, symlinks, non-regular files, malformed ODT packages, and oversized template files are rejected with generic template errors.
- The feature adds no telemetry, network access, accounts, cloud sync, remote catalogs, downloadable template packs, remote assets, or heavy dependencies.

## Deferred

- Rich template browsing, previews, categories, search, import/export of template packs, downloadable catalogs, account-backed libraries, cloud sync, and remote template assets.

## Verification

- `cargo test -p nine-hundred-word template`
- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `git diff --check`
