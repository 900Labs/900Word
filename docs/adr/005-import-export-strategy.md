# ADR 005: Import And Export Strategy

## Status

Accepted

## Decision

Rust owns import/export. The initial native format is ODT, with TXT, sanitized HTML, print HTML, basic PDF export, and bounded DOCX conversion. DOCX is import/export only and must pass through `word-core`; it is not a native save format. Binary `.doc` is deferred.

Sprint 033 keeps that boundary while adding simple DOCX page-region conversion for default and first-page headers/footers plus page-number, page-count, and date fields. Sprint 034 extends the same conversion-only boundary to safe embedded DOCX image relationships under `word/media/`, using existing `word-core` assets and `ImageBlock`s. Header/footer and image parts are local package parts only; remote targets, unsafe relationship targets, linked images, image sizing/cropping/layout fidelity, image compression/downsampling, even-page regions, complex fields, complex section layouts, comments, tracked changes, notes, broad media fidelity, and full layout fidelity remain outside the accepted DOCX conversion scope.

## Consequences

- Imported HTML is sanitized before reaching the frontend.
- Exported HTML and print HTML are generated from `word-core`, not from raw imported HTML.
- Export-to-path commands validate format-specific extensions and return only format/byte-count summaries to the frontend.
- DOCX imports open as dirty unsaved documents so Save continues to mean ODT unless the user chooses Save As.
- External converters are not bundled in the bootstrap.
