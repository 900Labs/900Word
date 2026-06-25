# Sprint 032: PDF Pagination And Export Settings MVP

## Status

Complete.

## Goal

Upgrade PDF export from a one-page text stream to a lightweight deterministic pagination and export-settings MVP while keeping 900Word local-first, low-resource, offline, ODT-canonical, and privacy-safe.

## Shipped

- Added typed PDF export options in `word-export` with a compatibility wrapper for existing basic PDF callers.
- Added deterministic lightweight text pagination using `word-core` page setup for page size and margins.
- Generated valid multi-page PDF output with one PDF page object per generated page and a generated xref table.
- Honored explicit `Block::PageBreak` as a PDF page break and prevented generated body text from flowing below the bottom margin.
- Repeated simple header/footer text on generated PDF pages and rendered page-number, page-count, and date fields deterministically.
- Kept paragraphs, headings, lists, tables, table-of-contents text, note references/bodies, and image alt/caption text in the PDF pagination path.
- Added PDF page-range options to desktop export and backend validation for invalid or empty ranges.

## Compatibility Boundary

- ODT remains canonical. PDF export is a conversion snapshot and does not update the current save path.
- PDF layout is text-oriented and deterministic, not a full editor-preview or print-layout engine.
- Page fields in generated PDFs use the lightweight generated pagination model.
- Range validation errors are generic and do not include local paths, private filenames, usernames, hostnames, or document text.

## Deferred

- Raster image embedding in PDF.
- Embedded or subset fonts.
- Active PDF link annotations.
- Complex script shaping.
- Page-bottom footnote layout and note continuation.
- Editor-preview layout fidelity and full office-suite PDF layout compatibility.

## Verification

- `cargo test -p word-export`
- `cargo test -p nine-hundred-word pdf`
- `npm --workspace apps/desktop run check`
- `npm --workspace apps/desktop run lint`
- `npm --workspace apps/desktop run test`
- `./scripts/verify-public-release.sh`
- `git diff --check`

## Privacy Notes

- Generated PDFs do not include local path metadata, source filenames, usernames, hostnames, creation-date metadata, producer metadata, telemetry identifiers, remote resources, or private build metadata.
- Desktop PDF settings are local UI state only and are not written into document content or metadata.
- Backend PDF range validation returns generic errors and exposes only export format and byte length on success.
