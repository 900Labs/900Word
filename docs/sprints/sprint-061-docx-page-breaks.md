# Sprint 061: DOCX Explicit Page Break Interoperability

## Goal

Add bounded DOCX import/export support for explicit page-break runs already represented by the local `PageBreak` block model without expanding into deterministic pagination or Word layout fidelity.

## Completed

- Imported top-level body paragraph `w:br w:type="page"` runs as local `PageBreak` blocks.
- Kept ordinary DOCX `w:br` runs, tracked-change page breaks, and table-cell page breaks as visible inline spacing.
- Exported local `PageBreak` blocks as generated DOCX `w:br w:type="page"` runs.
- Added import and export/import round-trip tests in `word-docx`.

## Boundaries

- ODT remains the canonical saved format.
- DOCX remains conversion-only; opening `.docx` creates an unsaved dirty document.
- Only explicit top-level body paragraph page-break runs are converted to page-break blocks.
- Layout-generated page markers, column-break semantics, line-clear semantics, page-break paragraph flags, deterministic pagination, and full Word layout fidelity remain deferred.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-docx page_break`
- `cargo test -p word-docx line_break`
- `cargo test -p word-docx`
- `./scripts/verify-local.sh`

## Privacy Notes

- Imported DOCX page breaks are normalized only into local block structure.
- Exported DOCX page breaks are generated from local `word-core` content and do not include source layout metadata that 900Word does not model.
- The feature does not store or emit local paths, private filenames, usernames, hostnames, source document names, account/cloud identifiers, telemetry identifiers, network state, or document-content logs.
- The feature adds no telemetry, network calls, cloud sync, accounts, remote fetching, plugin behavior, or heavy dependencies.
