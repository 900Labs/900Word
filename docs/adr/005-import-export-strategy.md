# ADR 005: Import And Export Strategy

## Status

Accepted

## Decision

Rust owns import/export. The initial formats are ODT, TXT, sanitized HTML, and basic PDF export. DOCX is deferred until ODT is stable. Binary `.doc` is deferred.

## Consequences

- Imported HTML is sanitized before reaching the frontend.
- External converters are not bundled in the bootstrap.
