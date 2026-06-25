# Sprint 034: DOCX Image Media

## Status

Complete.

## Scope

Sprint 034 improves the conversion-only DOCX boundary for embedded image media:

- Import safe local DOCX image relationships into existing `word-core` `AssetRef` values and `ImageBlock`s.
- Accept only relationship-resolved package parts under `word/media/` for PNG, JPEG/JPG, GIF, and WebP.
- Validate image payloads by package preflight, relationship target, extension/media type, and magic bytes before storing bytes.
- Preserve adjacent visible text around simple DOCX drawings by splitting mixed text/image paragraphs into adjacent text and image blocks when needed.
- Export valid 900Word-authored image assets as generated embedded DOCX media parts with document relationships, content type defaults, minimal DrawingML references, and alt text.
- Keep ODT as the canonical saved format.

## Implementation Notes

- Imported DOCX image assets receive generated IDs such as `docx-image-1.png`; source relationship target names and private source filenames are not stored in `original_name`.
- Unsafe, external, missing, unsupported, mismatched, or over-limit image relationships and payloads degrade with generic warnings.
- DOCX export writes generated `word/media/900word-image-<n>.<ext>` parts only for valid in-document image assets whose declared media type, byte length, package size budget, and magic bytes agree.
- Captions remain visible as nearby text on DOCX export when present.

## Verification

- Added a synthetic DOCX import test for an embedded PNG relationship producing one `AssetRef` and one `ImageBlock`.
- Added an import test proving adjacent text before and after a drawing is preserved.
- Added hostile relationship target coverage for traversal, absolute, drive-like, backslash, external, URL-like, and unsupported-extension image targets with generic warning behavior.
- Added a DOCX export/import round-trip test for a 900Word-authored image asset.
- Added package structure assertions for exported media parts, content type defaults, image relationships, drawing references, alt text, and visible captions.

## Deferred

- Linked or remote images.
- Image sizing, cropping, wrapping, anchoring, and full layout fidelity.
- Image compression, downsampling, or transcoding.
- DOCX comments, tracked changes, footnotes, and endnotes.
- Broad DOCX media fidelity beyond safe embedded raster image parts.
- Cloud, telemetry, accounts, network behavior, external converters, and heavyweight dependencies.
