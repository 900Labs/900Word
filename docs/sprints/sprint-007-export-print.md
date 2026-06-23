# Sprint 007: Export And Print

## Status

Complete.

## Delivered

- Added backend export-to-path commands for `.txt`, `.html`, and `.pdf` with format-specific extension validation, traversal rejection, atomic writes, and format/byte-count frontend results.
- Added sanitized print HTML generation from `word-core` and a desktop print action that uses the WebView print path.
- Hardened HTML export with generated offline CSP metadata, escaped text, safe inline mark rendering, unsafe-link stripping, no script/event-handler emission, and no remote image loads.
- Replaced the bootstrap PDF byte stub with a minimal valid PDF writer that emits page size metadata, text lines, xref data, `startxref`, and EOF.
- Added exporter tests for structural TXT output, sanitized HTML links/images, print page setup CSS, and PDF smoke validity.
- Documented that deterministic pagination, embedded fonts, complex script shaping, and layout-engine fidelity remain deferred.

## Validation

- `./scripts/verify-local.sh`
- `npm run check`
- `npm run lint`
- `npm run test`
- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo test -p word-export`

## Follow-Ups

- Add native save/export dialogs after dialog permissions and picker-granted file scopes are reviewed.
- Replace the text-oriented PDF adapter with an accepted layout/font strategy before claiming faithful pagination or complex script output.
- Add end-to-end print smoke coverage once browser/WebView automation is introduced.
