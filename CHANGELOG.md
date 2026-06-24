# Changelog

All notable changes to 900Word are tracked here.

## 0.1.0 - Unreleased

### Added

- Repository bootstrap with Rust workspace, Tauri/Svelte desktop shell, public documentation, validation scripts, and governance scaffolding.
- Initial `word-core`, `word-odf`, `word-spell`, `word-export`, and `word-fixtures` crates.
- TXT, sanitized HTML, basic PDF export-to-path workflows, and sanitized WebView print preparation.
- Release hardening scripts for bundle budgets, package artifact scans, runtime offline source/capability checks, SBOM generation, and performance smoke.
- Public-release privacy scanning for source files.
- Heading navigator sidebar and safe hyperlink insert/edit/remove workflows.
- Editable table projection for supported table cells and a default 2x2 insert-table command.
- Section-level header/footer MVP with simple page fields for page number, total page count, and date.
- Local image insertion MVP for PNG, JPEG, GIF, and WebP files with embedded document assets, ODT round-trip support, and offline HTML data-URL export.
- Image polish MVP with editable alt text, captions, alignment, and scale metadata preserved through `word-core`, ProseMirror text sync, ODT save/reopen, and sanitized HTML/print HTML export.
- Bookmarks and internal links MVP with safe generated block IDs on paragraphs/headings, compact editor controls, ODT `text:bookmark` round-trip, and sanitized HTML element IDs/fragment hrefs.
- Table structure editing MVP with bounded row/column insert sizes and contextual add/delete row, add/delete column, and delete-table controls for editable tables.
- Image resize UX with a direct selected-image drag handle that updates durable bounded scale metadata, plus generic oversized-image import guidance that avoids path and filename disclosure.
