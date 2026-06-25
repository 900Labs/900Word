# Sprint 049: Plain-Text Tabular Paste MVP

## Status

Complete.

## Goal

Add a conservative plain-text tabular paste path so users can paste simple spreadsheet-like tab-separated text into editable 900Word tables. Keep `.odt` canonical, `word-core` as durable truth, ProseMirror as the constrained editor projection, and avoid network, telemetry, accounts, cloud behavior, heavy parser dependencies, or document-content logging.

## Shipped

- Added TSV detection to the existing multiline plain-text paste helper in the desktop editor.
- Inserted supported ProseMirror table nodes when pasted text has at least two rows, at least one tab, and dimensions within the existing 1-8 row and 1-8 column bounds.
- Normalized CRLF/CR line endings to LF before detection.
- Preserved list-paste priority and native paste handling for partial multiline replacements inside non-empty paragraphs.
- Created pasted cells through ProseMirror text nodes only; no raw HTML is generated or parsed.
- Kept empty pasted cells editable by creating normal empty paragraph content with `sourceEmpty: false`.

## Compatibility Boundary

- One-cell-short rows are padded with editable empty trailing cells.
- Rows that are more than one cell short, blank interior rows, over-bound row/column counts, and non-tab multiline text fall back to the existing paragraph paste behavior.
- Bullet and numbered line paste keeps the existing list path before TSV detection.
- The feature changes editor paste behavior only; it does not add new document metadata or a new file-format model.

## Deferred

- Rich HTML clipboard import, formula handling, merged cells, table sizing, cell-type inference, spreadsheet parser dependencies, external spreadsheet fidelity, and broad table style import.

## Verification

- `npm --workspace apps/desktop run test -- editor documentProjection`
- `npm --workspace apps/desktop run check`
- `cargo fmt --all -- --check`
- `git diff --check`

## Privacy Notes

- Pasted text is handled in memory through the local editor transaction path and stored only as user document content.
- The feature does not log document or clipboard text, send network requests, add telemetry, create accounts, use cloud sync, preserve local paths or filenames, read usernames or hostnames, or introduce a spreadsheet parser dependency.
