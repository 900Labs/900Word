# ADR 003: word-core As Canonical Document Model

## Status

Accepted

## Decision

`word-core` is the durable source of truth. ProseMirror is an editing projection. ODT is the persisted package format.

## Consequences

- UI operations become `DocumentCommand` values.
- Import/export crates convert through `word-core`.
- Tests can validate document behavior without launching the desktop app.
