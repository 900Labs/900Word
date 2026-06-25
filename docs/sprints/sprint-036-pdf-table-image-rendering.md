# Sprint 036: PDF Table And Image Rendering

## Status

Complete.

## Goal

Improve the lightweight generated PDF export so simple tables and image presentation blocks render as visible document content while keeping ODT canonical and avoiding a heavy layout engine.

## Shipped

- Added a bounded PDF body projection for text lines, simple table rows/cells, image figure placeholders, and explicit page breaks.
- Paginated structured PDF body items by measured point height using existing page setup, header/footer, page-field, section, and page-range behavior.
- Rendered simple tables as vector cell boxes with wrapped cell text.
- Rendered image blocks as visible placeholder boxes using `ImageBlock` alt text, caption, alignment, and scale percentage metadata.
- Preserved the lightweight string/object PDF writer and deterministic low-resource behavior.

## Compatibility Boundary

- ODT remains the canonical saved format. PDF export is still a conversion snapshot and does not update the current save path.
- Table rendering is limited to simple row/cell boxes and visible cell text.
- Figure rendering is a placeholder representation only. It does not embed raster image bytes or claim image fidelity.
- PDF output does not emit image asset IDs, original source names, local paths, usernames, hostnames, creation-date metadata, producer metadata, telemetry identifiers, or remote resources as image metadata.

## Deferred

- Raster image embedding in PDF.
- Merged cells, table resizing, rich table styling, formulas, and complex nested table layout.
- Embedded or subset fonts.
- Active PDF link annotations.
- Complex script shaping.
- Page-bottom footnote layout and note continuation.
- Editor-preview layout fidelity and full office-suite PDF layout compatibility.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-export`
- `cargo test --workspace`
- `./scripts/verify-public-release.sh`
- `./scripts/verify-local.sh`
- `git diff --check`
