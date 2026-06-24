# Sprint 019: Template Gallery

## Status

Complete.

## Goal

Make a fresh 900Word install feel useful immediately for common lightweight writing tasks while keeping templates privacy-safe, generated in code, and compatible with the existing `word-core` and ODT save path.

## Shipped

- Preserved the existing `blank`, `report`, and `letter` template IDs for compatibility.
- Updated `report` into a school report starter and `letter` into a formal letter starter.
- Added generated starters for project reports, CV/resume documents, meeting minutes, memos, invoice-style documents, and flyer one-pagers.
- Used only supported durable blocks: headings, paragraphs, lists, and tables.
- Added table-based project, meeting-minutes, and invoice sections without adding formulas, external assets, downloadable files, or a new template format.
- Added tests for the full stable ID list, generated placeholder-safe content, real table blocks in table-heavy templates, and generic rejection for unknown or path-shaped template IDs.

## Deferred

- Template preview cards or screenshots.
- User-authored custom template storage.
- Downloadable template packs or remote template catalogs.
- Rich invoice formulas, mail merge, legal contract logic, or organization-specific examples.
- Embedded images, logos, fonts, or disk-backed template assets.

## Verification

- `npm run check`
- `npm run lint`
- `npm run test`
- `cargo fmt --all -- --check`
- `cargo test --workspace`
- `git diff --check`
- `./scripts/verify-public-release.sh`

## Privacy Notes

- Templates are generated from code and contain placeholder-only text.
- No real user documents, personal names, local paths, organization names, private endpoints, or disk images are embedded.
- The sprint does not add telemetry, accounts, cloud sync, remote assets, external template files, or new save/export behavior.
