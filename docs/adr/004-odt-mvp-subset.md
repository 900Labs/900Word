# ADR 004: ODT MVP Subset

## Status

Accepted

## Decision

The MVP ODT subset covers paragraphs, headings, inline formatting, lists, tables, page breaks, allowlisted embedded images, metadata, and named styles.

Unsupported content is preserved only when safe and structurally understood. Otherwise the document opens read-only with warnings or save is blocked.

Sprint 003 implements this as a bounded package round-trip for the current `word-core` model:

- `word-odf` writes `content.xml`, `meta.xml`, `META-INF/manifest.xml`, and embedded image payload entries.
- `word-odf` reads supported package content into `word-core` blocks, styles, list definitions, assets, and warnings.
- Image support requires in-memory `AssetRef` bytes and is limited to PNG, JPEG, GIF, and WebP payloads validated by magic bytes. Metadata-only image references are not treated as persisted image data.
- Text links are limited to `http`, `https`, and `mailto` during import.
- Unsupported ODT elements import with warnings instead of silent drops.
- Remote image references, path traversal, invalid mimetype entries, XML entities, unsafe archive entries, and executable/script entries are rejected or warned according to the file-format security policy.

## Consequences

- Silent data loss is not acceptable.
- ODT compatibility grows through golden fixtures and explicit sprint records.
- Full pagination, broad office-suite layout fidelity, macros, remote objects, and legacy binary formats remain outside the MVP subset.
