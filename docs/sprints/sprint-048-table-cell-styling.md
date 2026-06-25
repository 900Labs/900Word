# Sprint 048: Table Cell Styling MVP

## Status

Complete.

## Goal

Add a bounded, durable table-cell styling MVP for editable 900Word tables: basic cell background, per-cell text alignment, and border visible/hidden state. Keep `.odt` canonical, preserve `word-core` as the durable truth, avoid heavy table/layout abstractions, and keep unsupported imported table styling graceful.

## Shipped

- Added `word-core` table-cell presentation metadata for bounded light backgrounds, optional cell text alignment, and border visible/hidden state with backward-compatible defaults.
- Extended ProseMirror table-cell attrs and projection sync so supported cell styling round-trips between the editor and `word-core`.
- Added compact contextual toolbar controls that appear only when the selection is inside a supported editable table cell.
- Preserved 900Word-authored cell presentation through ODT save/reopen using bounded `word900` metadata attributes on table cells.
- Reflected supported cell background, alignment, and hidden-border choices in sanitized HTML and print HTML exports.
- Extended the lightweight PDF table projection to draw supported light cell fills, suppress hidden cell borders, and position left/center/right cell text. Justified cell text falls back to left-positioned wrapped text in the lightweight PDF renderer.

## Compatibility Boundary

- Only 900Word-authored cell presentation metadata is preserved as structured table styling.
- Unsupported or complex imported external table styling remains outside the MVP and is not claimed as lossless.
- Existing unstyled table cells remain default-visible borders with inherited text alignment and no fill.
- The editor still supports only rectangular tables whose cells contain supported paragraph, heading, or list content.

## Deferred

- Merged cells, formulas, cell sizing, table layout engine, rich style inheritance, table themes, arbitrary colors, border widths, border colors, per-side borders, rich paste, nested editable tables, and full external ODF/DOCX table-style interoperability.
- Exact PDF text justification and full office-suite pagination/layout fidelity.

## Verification

- `cargo test -p word-core table_cell_presentation`
- `cargo test -p word-odf table_cell_presentation`
- `cargo test -p word-export table_cell_styling`
- `npm --workspace apps/desktop run test -- documentProjection editorSchema editor i18n`
- `npm --workspace apps/desktop run check`
- `cargo fmt --all -- --check`
- `git diff --check`

## Privacy Notes

- Cell styling is stored only as bounded document presentation metadata inside the local document model and saved package.
- The feature does not add telemetry, accounts, cloud sync, remote fetches, network calls, local paths, filenames, usernames, hostnames, source document names, or document-content logging.
