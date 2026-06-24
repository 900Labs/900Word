# Sprint 018: Image Resize UX

## Status

Complete.

## Goal

Improve local image usability without widening the architecture. `word-core` remains durable truth, ProseMirror remains a projection, ODT remains the native saved format, and the slice does not add telemetry, accounts, cloud sync, remote assets, or a heavy image layout engine.

## Shipped

- Added a direct bottom-right resize handle for selected image atom blocks in the editor.
- Kept direct resizing backed by the existing bounded `scalePercent` image presentation metadata.
- Reused the existing selected-image attribute transaction path so toolbar scale controls and direct resize stay in sync.
- Kept accepted image bytes embedded in `document.assets`; the editor still renders only allowlisted in-document data URLs.
- Added generic oversized-image import guidance that tells users to compress or resize before insertion without exposing source paths, filenames, usernames, or filesystem details.
- Added focused tests for direct resize scale bounds, localized oversized guidance, and oversized import error privacy.

## Deferred

- Automatic compression or downsampling.
- Crop, rotation, native pixel sizing, and richer external ODT image-layout fidelity.
- Drag/drop image insertion.
- Header/footer/table-cell image editing.
- Raster image embedding in basic PDF export.

## Verification

- `npm run check`
- `npm run lint`
- `npm run test`
- `npm run build`
- `cargo fmt --all -- --check`
- `cargo test --workspace`
- `git diff --check`
- `./scripts/verify-public-release.sh`

## Privacy Notes

- No source local paths or source filenames are stored or shown for image imports.
- Oversized image rejection uses generic guidance only.
- Image resizing updates user-authored presentation metadata and does not add remote loading, telemetry, document logging, accounts, cloud sync, or external asset references.
