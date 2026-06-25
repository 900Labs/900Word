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
- Expanded generated starter template gallery for formal letters, school reports, project reports, resumes, meeting minutes, memos, invoice-style documents, and flyers using only placeholder-safe document model blocks.
- Editor viewport controls for Draft/Page Layout, Fit Width/100%/custom zoom, and simple show/hide rulers without adding document metadata or deterministic pagination claims.
- Keyboard shortcut polish with tested command normalization, guarded form-field behavior, visible shortcut hints, replace focus, Cmd/Ctrl+Y redo, and PDF export shortcut routing through the existing export path flow.
- Comments MVP with bounded local comment threads, privacy-safe `Local User` author defaulting, inline text anchors, a compact comments sidebar, resolve/reopen/delete controls, visible editor markers, and 900Word-authored ODT annotation round-trip support.
- Track Changes MVP with local Record changes state, visible text insertion/deletion marks, individual and bulk accept/reject controls, privacy-safe `Local User` authorship, timestamps, and 900Word-authored ODT round-trip support through `word900` metadata.
- Table of Contents MVP with a durable generated `word-core` block, H1-H3 heading-derived entries, safe generated bookmark targets, a local insert/update menu command, editor rendering as a linked contents block, and 900Word-authored ODT preservation through `word900` metadata without page-number fidelity claims.
- Footnotes/Endnotes MVP with bounded local note metadata, inline ProseMirror note-reference atoms, compact desktop insert controls, a Notes sidebar for stored note bodies, 900Word-authored ODT `text:note` round-trip, unsupported-note fallback warnings, and conservative TXT/HTML/print HTML/basic PDF note body export without page-bottom layout claims.
- Autocorrect and Smart Typing MVP with local disabled-by-default settings for sentence capitalization, smart quotes, double-hyphen em dashes, allowlisted typo replacements, and simple `- ` / `1. ` list triggers that apply only to user typing in the editor.
- Expanded Stats Panel MVP with a compact bottom-toolbar document information panel for word/character/paragraph counts, estimated pages, estimated reading time, selection words, comments, track changes, images/assets, footnotes/endnotes, and page size without adding document metadata or pagination fidelity claims.
- Accessibility and Low-Resource Mode MVP with disabled-by-default local settings for larger toolbar controls, reduced motion, and low-resource UI simplification without adding document metadata, export changes, telemetry, network access, cloud sync, accounts, or new dependencies.
- Recovery Snapshots MVP with versioned opaque recovery tokens, owner-only local recovery writes on Unix, dirty unsaved recovery opens, validated discard scope, and bounded retention of 3 snapshots per document and 20 snapshots overall.
