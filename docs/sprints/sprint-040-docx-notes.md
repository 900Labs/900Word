# Sprint 040: DOCX Footnotes/Endnotes MVP

## Status

Complete.

## Goal

Extend the conversion-only DOCX boundary so simple DOCX footnotes/endnotes map to existing `word-core` notes, and 900Word-authored local footnotes/endnotes export to valid DOCX note parts while keeping ODT canonical, offline, bounded, and privacy-safe.

## Shipped

- Imported standard internal DOCX footnote and endnote relationships that safely resolve to `word/footnotes.xml` and `word/endnotes.xml`.
- Parsed simple note bodies made of paragraphs and runs with visible text, tabs, and line breaks.
- Mapped supported `w:footnoteReference` and `w:endnoteReference` markers in body paragraphs, list items, and table cells to `Inline::note_reference` plus `Document.notes`.
- Generated local safe note IDs such as `note-docx-footnote-1` and `note-docx-endnote-1` instead of preserving raw DOCX IDs.
- Ignored separator/continuation notes and degraded missing, unsafe, duplicate, malformed, unanchored, over-limit, or tracked-change-only note content with generic warnings and visible fallback markers where a body reference was present.
- Exported supported 900Word-authored local notes as generated `word/footnotes.xml` and `word/endnotes.xml` parts with generated relationship IDs, content type overrides, numeric DOCX note IDs, and simple body paragraphs.
- Emitted `w:footnoteReference` and `w:endnoteReference` runs at supported body reference points.

## Compatibility Boundary

- DOCX remains import/export conversion only; ODT remains the native saved format.
- DOCX notes support simple text-only note bodies plus tabs and line breaks.
- Import does not read notes from headers, footers, comments, tracked-change-only hidden contexts, external relationships, custom XML, or unsupported package locations.
- Export uses generated numeric DOCX note IDs and generated part/relationship metadata only.

## Deferred

- Deterministic page-bottom footnote placement.
- Continuation separator fidelity.
- Note cross references.
- Rich note formatting, layout, and custom note styles.
- Comments or tracked changes inside note bodies.
- Full Microsoft Word / LibreOffice / ONLYOFFICE note round-trip fidelity.

## Verification

- `cargo test -p word-docx`

## Privacy Notes

- Imported raw DOCX note IDs are not preserved as local note IDs.
- Warnings are generic and do not include package paths, relationship targets, raw private IDs, local paths, private filenames, usernames, hostnames, account metadata, or unsupported hidden note body text.
- Exported DOCX note IDs, relationship IDs, and note part names are generated and do not include local note IDs, source filenames, local paths, usernames, hostnames, account metadata, telemetry IDs, or private build metadata.
- The feature adds no telemetry, accounts, cloud sync, remote lookup, remote asset loading, external converters, or document-content logging.
