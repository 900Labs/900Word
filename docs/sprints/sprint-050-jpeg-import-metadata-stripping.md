# Sprint 050: JPEG Import Metadata Stripping MVP

## Status

Complete.

## Goal

Strip common private metadata carrier segments from local JPEG/JPG image imports before embedding image bytes in 900Word documents. Keep `.odt` canonical, `word-core` as durable truth, and avoid telemetry, accounts, cloud behavior, network access, heavy image-processing dependencies, pixel decoding, recompression, downsampling, or resizing.

## Shipped

- Added dependency-free JPEG marker scanning to the desktop local image import boundary.
- Stripped APP0-APP15 and COM marker segments from accepted local JPEG/JPG imports before storing image bytes in `document.assets`.
- Preserved accepted SOI/EOI markers, structural JPEG segments, scan headers, and entropy-coded image data.
- Rejected malformed or ambiguous metadata-bearing JPEGs with the existing generic unsupported-image error instead of storing partially rewritten bytes.
- Kept the existing pre-read 8 MiB import limit and rechecked the sanitized byte length before embedding.
- Left PNG, GIF, and WebP import behavior unchanged.

## Compatibility Boundary

- This is local JPEG/JPG import metadata stripping only.
- The import path does not decode pixels, interpret EXIF, selectively preserve EXIF fields, resize, downsample, recompress, rotate, crop, or optimize images.
- APP/COM marker segments after scan data starts are treated as ambiguous and rejected rather than partially stripped.
- Source paths and source filenames remain unstored; accepted assets still use generated `image-<UUID>.jpg` identifiers with `original_name` empty.
- ODT remains the canonical saved package format.

## Deferred

- Full JPEG validation, broad malformed-JPEG recovery, progressive/multi-scan compatibility guarantees, EXIF interpretation/selective preservation, image resizing, downsampling, recompression, crop/rotation, color-management handling, and richer image optimization.

## Verification

- `cargo test -p nine-hundred-word image_import`
- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `git diff --check`

## Privacy Notes

- Accepted local JPEG/JPG import bytes no longer preserve pre-scan APP/COM payloads such as common EXIF/comment metadata carrier segments.
- Import errors stay generic and do not include source paths, filenames, metadata payload text, usernames, hostnames, account identifiers, telemetry identifiers, network state, or document text.
- The feature adds no network calls, telemetry, accounts, cloud sync, remote image fetching, external converters, or heavy image-processing dependency.
