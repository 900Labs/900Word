# Sprint 053: DOCX Table Width Interoperability

## Status

Complete.

## Goal

Carry the Sprint 052 bounded table column width model through the lightweight DOCX converter while keeping ODT canonical and DOCX conversion-only.

## Shipped

- Imported simple DOCX `w:tblGrid` / `w:gridCol w:w` width hints for editable rectangular tables with 1-8 columns.
- Normalized positive DOCX grid widths into per-mille `word-core` `Table.column_widths` values.
- Reused `word_core::sanitize_table_column_widths` as the final acceptance boundary.
- Exported valid 900Word-authored `Table.column_widths` as minimal generated DOCX `w:tblGrid` hints.
- Ignored invalid, mismatched, duplicate, zero, overflowed, missing, merged-cell, nested-table, or unsupported DOCX width metadata without surfacing raw values.

## Compatibility Boundary

- ODT remains the canonical saved format.
- DOCX remains a conversion-only import/export format.
- DOCX grid values are treated as proportional hints, not exact layout measurements.
- Missing or unsupported width metadata falls back to the existing equal-width behavior.
- Full table layout fidelity across office suites is not claimed.

## Privacy And Safety

- Width metadata is stored only as bounded integers in the document model.
- Exported DOCX grid hints are generated from sanitized 900Word-authored widths.
- The converter does not preserve arbitrary CSS, local paths, filenames, usernames, hostnames, source document names, account/cloud identifiers, telemetry identifiers, network state, or document text logs.
- The feature adds no telemetry, network behavior, cloud sync, accounts, remote resource loading, or heavy dependencies.

## Deferred

- Full table layout engine, drag UI, merged cells, formulas, arbitrary CSS, rich table themes, spreadsheet import, remote behavior, telemetry, accounts, cloud sync, and heavy dependencies.

## Verification

- Independent builder and reviewer agent passes completed before control-tower acceptance.
- `./scripts/verify-local.sh`
- `cargo fmt --all -- --check`
- `cargo test -p word-docx table`
- `cargo test -p word-docx`
- `cargo check --workspace`
- `git diff --check`
