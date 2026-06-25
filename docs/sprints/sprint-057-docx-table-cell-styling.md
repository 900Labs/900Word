# Sprint 057: DOCX Table Cell Presentation Interoperability

## Goal

Add bounded DOCX import/export support for the existing 900Word table-cell presentation model without broadening into full DOCX table themes or layout fidelity.

## Completed

- Imported safe DOCX `w:shd` cell fills when they map to the existing 900Word light color palette.
- Imported hidden cell borders only when all four DOCX cell sides are explicitly `nil` or `none`.
- Derived cell text alignment only when supported paragraph-like content in the table cell explicitly agrees on simple `w:jc` values.
- Exported 900Word-authored safe cell fills, hidden border markers, and simple cell text alignment in generated DOCX table cells.
- Added round-trip and unsupported/mixed presentation tests in `word-docx`.

## Boundaries

- ODT remains the canonical saved format.
- DOCX remains conversion-only; opening `.docx` creates an unsaved dirty document.
- Unsupported colors, partial/per-side borders, border widths, border colors, rich table themes, merged cells, formulas, deterministic table layout, and full DOCX table style fidelity remain deferred.
- Mixed paragraph alignments in a DOCX table cell are not promoted to the cell-level 900Word presentation model.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-docx`
- `cargo clippy -p word-docx -- -D warnings`
- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `./scripts/verify-local.sh`

## Privacy Notes

- The DOCX converter writes only generated safe color constants, `nil` border markers, and simple alignment values for cell presentation.
- The feature does not store or emit local paths, private filenames, usernames, hostnames, source document names, account/cloud identifiers, telemetry identifiers, network state, or document-content logs.
- The feature adds no telemetry, network calls, cloud sync, accounts, remote fetching, plugin behavior, or heavy dependencies.
