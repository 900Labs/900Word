# Sprint 027: Expanded Stats Panel MVP

## Status

Complete.

## Goal

Expand the desktop word count/statistics panel into a bounded local document information panel while keeping 900Word lightweight, offline, ODT-native, public-release safe, and free of cloud, accounts, telemetry, network behavior, AI services, heavyweight dependencies, new document metadata, or export/import changes.

## Shipped

- Added a compact expanded Stats panel behind the existing bottom-toolbar Stats button.
- Kept the bottom toolbar always visible and limited the persistent footer values to words, selection words, and estimated pages.
- Added expanded document counts for words, characters with spaces, characters without spaces, paragraphs, model blocks, estimated pages, estimated reading time, and selection word count.
- Labelled page count and reading time as estimates and avoided deterministic pagination claims.
- Added local-first document indicators already available in the model: comments, unresolved comments, track changes status, tracked changes count, images, embedded assets, footnotes, endnotes, and page size.
- Added a small tested `documentStats` helper module for the expanded stats projection.
- Kept the feature UI-only with no ODT metadata changes, export changes, import changes, network access, telemetry, cloud sync, accounts, or new dependencies.

## Compatibility Boundary

- Estimated pages use a lightweight words-per-page estimate. They are not layout pages and do not imply deterministic pagination, page-break preview, font metrics, print layout fidelity, or ODT page-number fidelity.
- Estimated reading time uses a simple words-per-minute estimate and is not user-adaptive.
- Selection word count uses the existing editor selection snapshot/plain-text path and reports zero when the current selection is empty or unavailable.
- Comment, track-change, note, image, asset, and page-size indicators are aggregate UI values only. The panel does not show local paths, private filenames, usernames, hostnames, source image filenames, or recovery locations.
- Paragraph count is based on supported projected paragraph and heading content, including supported list/table child blocks. It is not a full external ODT layout analysis.

## Deferred

- Deterministic pagination, page-break preview, word count by section, exportable statistics reports, readability analysis, per-language reading speeds, and document metadata persistence.
- A richer statistics sidebar or modal.
- Full component-level browser tests for the Svelte panel; the MVP covers the projection helper plus Svelte/TypeScript checks.

## Verification

- `npm --workspace apps/desktop run test -- documentStats` - passed.
- `npm --workspace apps/desktop run test -- documentStats i18n` - passed.
- `npm --workspace apps/desktop run check` - passed.
- `npm --workspace apps/desktop run lint` - passed.
- `npm --workspace apps/desktop run build` - passed.
- `git diff --check` - passed.

## Privacy Notes

- The panel derives aggregate values from existing local document state and the existing editor selection snapshot.
- It does not add telemetry, accounts, cloud sync, remote lookup, AI services, network behavior, document-content logging, local path display, private filename display, username access, hostname access, source image filename display, export metadata, or saved document metadata.
