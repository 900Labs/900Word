# Sprint 012: Editable Tables MVP

Status: complete.

## Scope

This sprint adds a focused desktop editing path for existing `word-core` table blocks without changing the durable Rust table model, adding network behavior, or widening import/export claims.

## Completed In This Slice

- Added ProseMirror `table`, `table_row`, and `table_cell` nodes that map to existing `word-core` `Table`, `TableRow`, and `TableCell` structures.
- Projected table cells containing paragraphs, headings, or lists into editable desktop table cells.
- Synced edited supported table cell content back through the existing `apply_document_command` block replacement path.
- Added a toolbar command that inserts a default 2x2 table at the current top-level editor selection.
- Added tests for schema support, document-to-editor table projection, editor-to-`word-core` table sync, insert-table transactions, and unsupported table-cell content.

## Current Limits

- Cells containing unsupported blocks, such as images or nested tables, are shown with a placeholder and make the editor projection read-only so the original model is preserved.
- Source tables with no rows, or rows with no cells, remain read-only until structural table editing is implemented.
- Existing supported lists inside table cells round-trip, but using toolbar list commands to create new lists inside cells remains deferred.
- Inserted tables start as a simple 2x2 grid with empty body paragraphs.
- Row/column insertion, delete-table controls, merged cells, cell sizing, table styles, captions, and rich table paste remain deferred.
- Table cell projection supports paragraphs, headings, and lists only. Other modeled blocks need their own durable editor behavior before they become editable inside cells.

## Verification

- `npm run check`
- `npm run lint`
- `npm run test`
- `npm run build`
- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `./scripts/verify-local.sh`
- `git diff --check`

## Privacy Notes

- No telemetry, cloud calls, accounts, remote fetches, screenshots, local paths, private filenames, or real documents were added.
- Table editing runs entirely through the local ProseMirror projection and existing local document command path.
- Unsupported cell content is not serialized into DOM attributes or docs; the editor shows a generic local placeholder instead.
