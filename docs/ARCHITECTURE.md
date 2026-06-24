# Architecture

900Word is a local-first desktop application with a Rust-owned document core and a web-based editor projection.

## System Shape

```text
Svelte 5 UI + ProseMirror
        |
        | typed Tauri IPC
        v
Tauri commands in apps/desktop/src-tauri
        |
        v
Rust workspace crates
  - word-core: canonical document model
  - word-odf: ODT package read/write and validation
  - word-spell: dictionary boundary
  - word-export: TXT, HTML, PDF export
  - word-fixtures: sanitized generated fixtures
```

## Source Of Truth

`word-core` owns durable document truth. ProseMirror is an editing projection, and ODT is the persisted package format.

The editor schema accepts only nodes and marks with matching `word-core` semantics. Sprint 009 adds editable ordered/unordered list nodes, paragraph direct-format attributes, and inline text-style attributes on top of the original paragraph, heading, text, and inline mark projection. Sprint 010 adds selection-derived toolbar state, list-aware Enter/paste behavior, and a durable update-style-from-selection command for selected paragraph style properties. Sprint 011 adds a live Heading 1/2/3 navigator derived from projected `word-core` blocks and exposes insert/edit/remove hyperlink UI for the existing safe link mark. Sprint 012 adds an editable table projection for table cells containing paragraphs, headings, or lists, plus a default 2x2 insert-table toolbar command. Sprint 013 adds section-level header/footer page regions and typed page fields in `word-core`, while the desktop editor keeps them in Settings as a simple text-backed surface rather than projecting them into the ProseMirror body editor. Broader ProseMirror nodes remain unavailable until `word-core` has matching durable semantics and import/export tests. Documents that contain modeled-but-unprojected blocks, such as images or unsupported nested table-cell content, open in a read-only editor projection with warnings until those blocks have complete projection support.

Import flow:

1. Rust validates and parses the input file.
2. Rust converts supported content into `word-core`.
3. Rust emits warnings for unsupported content.
4. The frontend receives sanitized editor JSON, never raw imported HTML.

Editor sync flow:

1. ProseMirror emits a constrained editor JSON document.
2. The frontend converts supported editor nodes back into `word-core` blocks.
3. The frontend submits `DocumentCommand` values through Tauri IPC.
4. Tauri commands validate the request.
5. `word-core` applies changes and remains the durable source of truth.
6. `word-odf` writes the supported ODT subset when a save command runs.

## ODT Boundary

`word-odf` owns OpenDocument Text package validation and conversion for the current MVP subset. It does not expose raw imported XML or HTML to the frontend.

Current ODT support covers:

- Paragraphs and headings with `word-core` style IDs.
- Paragraph style registry properties for 900Word-authored paragraph style updates.
- Paragraph direct formatting for 900Word-authored alignment, line spacing, spacing, and indents.
- Inline text runs with bold, italic, underline, strikethrough, superscript, subscript, font family, font size, text color, highlight color, and safe `http`, `https`, or `mailto` links.
- Ordered and unordered lists with 900Word-authored list item levels.
- Tables with paragraph content inside cells.
- Page breaks as explicit `word-core` blocks.
- 900Word-authored headers, footers, optional first-page header/footer regions, and page-number/page-count/date fields.
- Metadata title read/write.
- Embedded PNG, JPEG, GIF, and WebP payloads through `AssetRef` bytes and `ImageBlock` references.

Package preflight enforces raw package size, entry count, entry size, expanded size, path depth, XML depth, image size, safe archive paths, symlink rejection, encrypted entry rejection, executable/script entry rejection, first-entry stored ODT mimetype validation, image magic-byte validation, and XML entity/doctype rejection. Unsupported ODT elements are imported with warnings. Unsafe text links are stripped with warnings. Remote or path-traversing image references are ignored with warnings. Unsupported external header/footer complexity imports with warnings and read-only page-region state so save operations refuse to silently flatten it.

## Fixtures

`crates/word-fixtures` contains generated fixtures only. JSON fixtures must use synthetic content, deterministic identifiers, and no real user documents. ODT round-trip tests generate package bytes in memory from synthetic document data rather than checking binary user documents into the repository.

## Deferred Systems

The following are not part of the bootstrap implementation:

- Binary `.doc` import/export.
- Cloud sync and real-time collaboration.
- Runtime plugin execution.
- Downloadable asset stores.
- Deterministic pagination engine.
- Full document encryption.

Each requires an accepted ADR before implementation.
