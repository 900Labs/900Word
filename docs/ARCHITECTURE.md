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

Import flow:

1. Rust validates and parses the input file.
2. Rust converts supported content into `word-core`.
3. Rust emits warnings for unsupported content.
4. The frontend receives sanitized editor JSON, never raw imported HTML.

Save flow:

1. The frontend submits document commands or a sanitized projection.
2. Tauri commands validate the request.
3. `word-core` applies changes.
4. `word-odf` writes the supported ODT subset.

## Deferred Systems

The following are not part of the bootstrap implementation:

- Binary `.doc` import/export.
- Cloud sync and real-time collaboration.
- Runtime plugin execution.
- Downloadable asset stores.
- Deterministic pagination engine.
- Full document encryption.

Each requires an accepted ADR before implementation.
