# ADR 005: Import And Export Strategy

## Status

Accepted

## Decision

Rust owns import/export. The initial native format is ODT, with TXT, sanitized HTML, print HTML, basic PDF export, and bounded DOCX conversion. DOCX is import/export only and must pass through `word-core`; it is not a native save format. Binary `.doc` is deferred.

## Consequences

- Imported HTML is sanitized before reaching the frontend.
- Exported HTML and print HTML are generated from `word-core`, not from raw imported HTML.
- Export-to-path commands validate format-specific extensions and return only format/byte-count summaries to the frontend.
- DOCX imports open as dirty unsaved documents so Save continues to mean ODT unless the user chooses Save As.
- External converters are not bundled in the bootstrap.
