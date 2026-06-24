# Sprint 013: Headers, Footers, And Page Fields

Status: complete.

## Scope

This sprint adds a bounded MVP for local, durable section page regions. It keeps `.odt` native, keeps `word-core` as the source of truth, and does not add telemetry, accounts, cloud sync, remote assets, or deterministic pagination.

## Completed In This Slice

- Added `word-core` section-level `PageRegions` with header, footer, first-page header, first-page footer, and a different-first-page toggle.
- Added typed inline page fields for page number, total page count, and date.
- Added document commands for updating page regions and toggling different-first-page behavior.
- Persisted 900Word-authored header/footer paragraphs and page fields through ODT `styles.xml`.
- Imported unsupported external header/footer complexity with warnings and read-only page-region state so save refuses silent flattening.
- Included header/footer text in TXT, HTML, print HTML, and basic PDF export.
- Added a Settings-panel editing surface with plain text controls and buttons that insert field tokens.
- Added focused Rust and frontend tests for commands, ODT round-trip/read-only behavior, export output, and token-backed UI helpers.
- Added a read-only projection guard for semantic page fields imported into the document body, where the current ProseMirror body editor cannot preserve field nodes.

## Current Limits

- Header/footer editing is plain text backed. It does not expose rich formatting, tables, images, or nested content in page regions.
- Page number and total page count export as `1` because deterministic pagination remains deferred.
- Date fields export as the document modified date.
- The ProseMirror body editor does not directly project page regions; Settings owns the MVP editing surface.
- Complex external ODT header/footer structures are preserved as read-only warning state and cannot be rewritten by this MVP.

## Verification

- `cargo fmt --all -- --check`
- `npm run check`
- `npm run lint`
- `npm run test`
- `npm run build`
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `./scripts/verify-public-release.sh`
- `./scripts/verify-local.sh`
- `git diff --check`
- Touched source/docs privacy scan for local paths, hostnames, and personal names

## Privacy Notes

- No telemetry, cloud calls, accounts, remote fetches, screenshots, local paths, private filenames, or real documents were added.
- Page fields render from the local document/export context only.
- Header/footer warnings are generic and do not expose imported document details beyond unsupported-structure status.
- The ODT custom metadata namespace uses a URN rather than a URL-like hostname.
