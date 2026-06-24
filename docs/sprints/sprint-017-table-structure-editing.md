# Sprint 017: Table Structure Editing MVP

## Status

Complete.

## Goal

Add a usable local-first table-structure editing slice for 900Word-authored tables while keeping ODT as the native format, `word-core` as durable truth, and ProseMirror as a constrained projection.

## Shipped

- Replaced the fixed 2x2 insert-table behavior with a compact bounded insert control for 1-8 rows by 1-8 columns.
- Added contextual table toolbar controls for add row above, add row below, delete row, add column left, add column right, delete column, and delete table.
- Kept table controls disabled and transaction helpers harmless when the selection is outside an editable table.
- Limited structural editing to rectangular editable tables whose cells contain supported paragraph, heading, or list content.
- Preserved the existing `word-core` table model and ODT round-trip boundary; row/column edits are projected through the existing editor sync path as updated table blocks.
- Added transaction tests for custom-size insertion, dimension bounds, row/column add-delete operations, delete-table fallback, and no-op behavior outside tables.

## Deferred

- Merged cells.
- Drag resizing, explicit column widths, and row heights.
- Formulas or spreadsheet-style table behavior.
- Heavy table styling and durable cell border/background editing.
- Rich table paste/import cleanup beyond the current supported ODT table subset.
- Editing unsupported nested content inside table cells.

## Verification

- `npm run check`
- `npm run lint`
- `npm run test`
- `cargo fmt --all -- --check`
- `cargo test --workspace`
- `git diff --check`
- `./scripts/verify-public-release.sh`

## Privacy Notes

- No telemetry, accounts, cloud sync, remote assets, or network behavior were added.
- Table editing stays inside local editor state and the existing local document command path.
- The UI and docs do not store local source paths, private filenames, usernames, or hostnames.
