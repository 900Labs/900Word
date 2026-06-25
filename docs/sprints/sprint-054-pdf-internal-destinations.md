# Sprint 054: PDF Internal Destinations

## Status

Complete.

## Goal

Add bounded active PDF destinations for safe 900Word internal `#bookmark` links and generated TOC entries without adding a layout engine, full PDF outline tree, remote fetching, telemetry, cloud behavior, accounts, or heavy PDF dependencies.

## Shipped

- Extended the existing PDF linked-text projection with typed link targets so safe external URI links and safe internal bookmark links share the same wrapping and annotation path.
- Collected generated PDF destination coordinates for unique safe paragraph/heading bookmark IDs based on the lightweight page, line, and table-cell positions where the bookmarked block appears.
- Emitted internal PDF `/Link` annotations with `/Dest` arrays for safe `#bookmark` links only when the target bookmark exists in the exported PDF pages.
- Added generated TOC PDF link annotations for entries whose safe target bookmark exists in exported PDF content.
- Preserved the existing per-page and per-document annotation caps. Over-budget internal links remain visible text only.
- Kept link rectangles bounded to approximate text-run areas and avoided page-wide clickable regions.
- Omitted unsafe fragments, missing targets, duplicate targets, and page-range-excluded targets without writing internal href strings to the PDF bytes.

## Implementation Notes

- Internal annotations use generated page-object destinations instead of `/URI` actions.
- Destination positions reuse the lightweight PDF cursor math. They are useful navigation anchors, not glyph-perfect layout coordinates.
- Page-range export builds destinations only for the selected PDF pages, so links to targets outside the range do not produce dangling annotations.
- Duplicate safe bookmark IDs are treated as ambiguous and do not become PDF destinations.
- The feature does not emit bookmark IDs, source paths, filenames, usernames, hostnames, source names, account/cloud identifiers, telemetry identifiers, document-content logs, creation metadata, or producer metadata.

## Review Coverage

- Added regression coverage for generated TOC entries that point at duplicate bookmark IDs.
- Added regression coverage for internal destinations that reference later PDF page objects after earlier selected pages emit both annotations and image XObjects.
- Added regression coverage for bookmark targets inside list items and table cells.
- Added regression coverage for mixed external URI, direct internal bookmark, and generated TOC annotations sharing the per-page annotation cap.

## Deferred

- Full PDF outline/bookmark trees.
- Exact glyph-level link geometry.
- Cross-document links.
- PDF comment annotations and note backlinks.
- Full office-suite PDF layout compatibility.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-export pdf_export`
- `cargo check --workspace`
- `git diff --check`
- `cargo clippy --workspace -- -D warnings`
- `./scripts/verify-local.sh`
