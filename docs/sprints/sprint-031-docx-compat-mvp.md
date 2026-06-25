# Sprint 031: DOCX Compatibility MVP

## Status

Complete.

## Goal

Add bounded `.docx` import/export conversion while keeping OpenDocument Text (`.odt`) as the canonical saved format and avoiding cloud, accounts, telemetry, network behavior, external converters, or heavy dependencies.

## Shipped

- Added `crates/word-docx`, a lightweight converter that reuses workspace `zip` and `quick-xml`.
- Added DOCX package preflight for size limits, entry count, expanded size, path depth, XML depth, unsafe paths, symlinks, encrypted entries, macro/executable-like entries, entity declarations, and required `word/document.xml`.
- Imported simple DOCX paragraphs, Heading 1-3 styles, bold/italic/underline runs, safe hyperlinks, simple list fallback, and simple tables into `word-core`.
- Added generic document warnings for unsafe hyperlinks, missing hyperlink targets, degraded numbering, nested table flattening, media/inline metadata skips, and unsupported body/paragraph content.
- Exported minimal valid `.docx` packages with document, styles, numbering, and relationship parts for paragraphs, headings, basic inline marks, safe hyperlinks, simple lists, and simple tables.
- Added Tauri IPC for `open_docx_document` and `export_docx_to_path` with backend `.docx` extension/traversal validation.
- Updated the desktop File menu so Open accepts `.odt` and `.docx`, while Save/Save As remain ODT-native and Export includes DOCX.

## Compatibility Boundary

- ODT remains canonical. DOCX import opens as dirty and unsaved, with no current save path adopted from the `.docx` source.
- DOCX export is a conversion snapshot and does not update the current save path.
- The MVP does not import or export structured DOCX media, comments, tracked changes, footnotes/endnotes, headers/footers, fields, advanced styles, merged cells, formulas, embedded objects, macros, custom XML, or deterministic layout.
- Unsupported DOCX content is ignored, flattened to visible text where practical, or surfaced through generic warnings. Warnings do not include local paths, private filenames, usernames, or hostnames.

## Deferred

- Broad DOCX compatibility for comments, tracked changes, notes, images, headers/footers, advanced numbering, style fidelity, merged cells, embedded objects, fields, page layout, and rich table semantics.
- Binary `.doc` import/export and any external converter integration.
- Real-world golden document corpus. Tests remain synthetic byte generation only for the MVP.

## Verification

- `cargo test -p word-docx` - required for synthetic DOCX import/export/package validation coverage.
- `cargo test -p nine-hundred-word docx` - required for desktop `.docx` path/export validation coverage.
- `npm --workspace apps/desktop run check` - required for Svelte/TypeScript IPC integration.
- `npm --workspace apps/desktop run test -- i18n` - required for localization type/string coverage.
- `git diff --check` - required for whitespace safety.

## Privacy Notes

- Source `.docx` paths are validated in Rust and are not stored in frontend document state.
- DOCX import warnings are generic and do not expose private filenames or local paths.
- DOCX export results expose only format and byte length.
- The feature adds no telemetry, accounts, cloud sync, remote lookup, remote resource fetching, AI services, external converter execution, document-content logging, username reads, hostname reads, or path metadata.
