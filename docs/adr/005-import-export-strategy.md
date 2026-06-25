# ADR 005: Import And Export Strategy

## Status

Accepted

## Decision

Rust owns import/export. The initial native format is ODT, with TXT, sanitized HTML, print HTML, basic PDF export, and bounded DOCX conversion. DOCX is import/export only and must pass through `word-core`; it is not a native save format. Binary `.doc` is deferred.

Sprint 033 keeps that boundary while adding simple DOCX page-region conversion for default and first-page headers/footers plus page-number, page-count, and date fields. Sprint 034 extends the same conversion-only boundary to safe embedded DOCX image relationships under `word/media/`, using existing `word-core` assets and `ImageBlock`s. Sprint 035 extends it to simple anchored DOCX comments using existing `word-core` comment threads and inline comment anchors. Sprint 056 extends it to unique safe paragraph/heading bookmark markers plus generated 900Word TOC paragraphs using `Word900Toc*` styles and safe internal links. Sprint 057 extends it to bounded safe table-cell presentation tags for the existing 900Word light-fill, hidden-border, and simple text-alignment model. Sprint 058 extends it to bounded safe paragraph formatting tags for the existing 900Word alignment, automatic line-spacing, spacing, and indent model. Header/footer, image, comments, bookmarks, generated TOC shapes, bounded cell presentation shapes, and bounded paragraph formatting shapes are local package content only; remote targets, unsafe relationship targets, linked images, image sizing/cropping/layout fidelity, image compression/downsampling, even-page regions, complex fields, complex section layouts, comment replies/threading/resolved-state fidelity, tracked changes, notes, Word TOC field-code fidelity, deterministic TOC page numbers, arbitrary paragraph settings, arbitrary table colors, per-side borders, rich table themes, broad media fidelity, full DOCX review fidelity, and full layout fidelity remain outside the accepted DOCX conversion scope.

## Consequences

- Imported HTML is sanitized before reaching the frontend.
- Exported HTML and print HTML are generated from `word-core`, not from raw imported HTML.
- Export-to-path commands validate format-specific extensions and return only format/byte-count summaries to the frontend.
- DOCX imports open as dirty unsaved documents so Save continues to mean ODT unless the user chooses Save As.
- External converters are not bundled in the bootstrap.
