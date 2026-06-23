# ADR 005: Import And Export Strategy

## Status

Accepted

## Decision

Rust owns import/export. The initial formats are ODT, TXT, sanitized HTML, print HTML, and basic PDF export. DOCX is deferred until ODT is stable. Binary `.doc` is deferred.

## Consequences

- Imported HTML is sanitized before reaching the frontend.
- Exported HTML and print HTML are generated from `word-core`, not from raw imported HTML.
- Export-to-path commands validate format-specific extensions and return only format/byte-count summaries to the frontend.
- External converters are not bundled in the bootstrap.
