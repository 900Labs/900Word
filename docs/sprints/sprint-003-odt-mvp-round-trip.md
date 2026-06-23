# Sprint 003: ODT MVP Round-Trip

## Scope

Implement a bounded OpenDocument Text package round-trip for the current `word-core` document model without claiming full office-suite layout compatibility.

## Deliverables

- `word-odf` writes ODT packages with `mimetype`, `content.xml`, `meta.xml`, `META-INF/manifest.xml`, and embedded image payload entries.
- `word-odf` reads supported ODT content into `word-core` documents.
- ODT package validation enforces raw package size, entry count, entry size, expanded size, path depth, XML depth, image size, path safety, symlink rejection, encrypted entry rejection, executable/script entry rejection, first-entry stored mimetype validation, image magic-byte validation, and XML entity/doctype rejection.
- Paragraphs, headings, inline marks, safe links, unordered/ordered lists, tables, page breaks, metadata title, named paragraph styles, and embedded image bytes round-trip through generated package bytes.
- RTL and CJK content are covered by synthetic test data.
- Unsupported ODT elements import with warnings.
- Unsafe text links are stripped with warnings.
- Remote or path-traversing image references are ignored with warnings.
- Unsupported image payload types are rejected.
- Image persistence is backed by `AssetRef` bytes in `word-core`.

## Validation

Run from the repository root:

```bash
cargo test -p word-odf
cargo test --workspace
cargo clippy --workspace -- -D warnings
./scripts/verify-public-release.sh
```

## Evidence

- `crates/word-odf/src/lib.rs` contains the ZIP/XML preflight, ODT writer, ODT reader, and package round-trip tests.
- `crates/word-core/src/lib.rs` stores image asset bytes so ODT packages can persist embedded image payloads.
- `docs/FILE_FORMATS.md` documents the current ODT support level and limits.
- `docs/FILE_FORMAT_SECURITY.md` documents implemented ODT package controls.

## Follow-Ups

- Sprint 004 owns user-facing open/save/save-as, autosave, crash recovery, and recent-file workflows.
- Broader ODT compatibility requires new hostile-file fixtures and explicit unsupported-content behavior before public claims expand.
