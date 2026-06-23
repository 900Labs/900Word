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

The Sprint 002 editor schema intentionally accepts only paragraph, heading, text, and supported inline mark projections. Broader ProseMirror nodes remain unavailable until `word-core` has matching durable semantics and import/export tests. Documents that contain modeled-but-unprojected blocks, such as tables or images, open in a read-only editor projection with warnings until those blocks have complete projection support.

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

## Fixtures

`crates/word-fixtures` contains generated fixtures only. JSON fixtures must use synthetic content, deterministic identifiers, and no real user documents.

## Deferred Systems

The following are not part of the bootstrap implementation:

- Binary `.doc` import/export.
- Cloud sync and real-time collaboration.
- Runtime plugin execution.
- Downloadable asset stores.
- Deterministic pagination engine.
- Full document encryption.

Each requires an accepted ADR before implementation.
