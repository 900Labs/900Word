# Sprint 038: PDF JPEG Image Embedding

## Status

Complete.

## Goal

Embed safe in-document JPEG/JPG assets in generated PDFs as lightweight raster image XObjects while keeping ODT canonical and preserving the existing visible placeholder fallback for unsupported image payloads.

## Shipped

- Added dependency-free baseline JPEG structure parsing for bounded 8-bit JPEG assets.
- Stripped APP/COM JPEG metadata marker segments before embedding image streams.
- Embedded safe JPEG assets already present in `word-core` document assets as PDF `/Subtype /Image` XObjects using `/Filter /DCTDecode`.
- Rendered embedded images inside the existing figure layout, respecting page size, margins, image alignment, and bounded scale percentage metadata.
- Kept figure alt text and captions visible when present.
- Preserved PDF page-range behavior so selected-page exports include only selected-page image XObjects.
- Kept PNG, GIF, WebP, malformed JPEG, oversized JPEG, unsupported component-count, and over-cap image cases on the existing visible figure-placeholder fallback path.
- Preserved the lightweight PDF writer and added binary-safe object serialization without introducing a new PDF or image-processing dependency.

## Compatibility Boundary

- ODT remains the canonical saved format. PDF export is still a conversion snapshot.
- Embedded PDF raster images are limited to in-document assets whose stored media type and detected magic bytes are `image/jpeg`.
- JPEG embedding is capped at 32 images per generated PDF, 8 MiB per embedded JPEG, 8192 px per side, 20,000,000 pixels, and grayscale or RGB component counts only.
- JPEG APP/COM metadata marker segments are stripped before embedding. JPEGs with metadata markers after scan data starts fall back to visible placeholders.
- Unsupported or malformed image payloads continue to render as visible figure placeholders with alt/caption text when present.
- PDF export does not load external files, remote URLs, linked images, source filenames, local paths, usernames, hostnames, asset IDs, creation-date metadata, producer metadata, APP/COM JPEG metadata, or rich image metadata.

## Deferred

- PNG, GIF, and WebP raster embedding in generated PDFs.
- Progressive JPEG embedding, JPEG decoding, validation beyond the bounded structure checks, downsampling, recompression, and compression tuning.
- Crop, rotation, EXIF interpretation/selective preservation, color-management precision, CMYK support, and rich PDF image metadata.
- Full layout fidelity, image wrapping, exact office-suite image placement, and a general PDF layout engine.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-export`
- `cargo test --workspace`
- `./scripts/verify-public-release.sh`
- `git diff --check`

Reviewer follow-up added APP/COM metadata stripping and corrupt-after-SOF fallback coverage before the final gate.
