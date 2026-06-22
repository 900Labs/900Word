# ADR 004: ODT MVP Subset

## Status

Accepted

## Decision

The MVP ODT subset covers paragraphs, headings, inline formatting, lists, tables, images, metadata, and named styles.

Unsupported content is preserved only when safe and structurally understood. Otherwise the document opens read-only with warnings or save is blocked.

## Consequences

- Silent data loss is not acceptable.
- ODT compatibility grows through golden fixtures and explicit sprint records.
