# Sprint 052: Table Column Widths MVP

## Status

Complete.

## Goal

Make simple editable tables more useful by adding bounded column width hints while keeping ODT canonical, `word-core` as the source of truth, and table layout fidelity explicitly limited.

## Shipped

- Added `word-core` table `column_widths` metadata as bounded per-mille integers.
- Limited valid width metadata to rectangular editable tables with 1-8 columns.
- Added schema/projection support so valid widths survive ProseMirror editing and sync back to `word-core`.
- Added a compact contextual selected-column width control in the existing table toolbar.
- Kept add/delete column operations metadata-consistent by inserting a default width, removing deleted-column widths, and normalizing the remaining hints.
- Preserved 900Word-authored widths through generated ODT with safe `word900:column-widths` metadata on table elements.
- Reflected valid hints in sanitized HTML/print HTML through generated `colgroup` percentages and in lightweight PDF table projection through proportional cell widths.

## Compatibility Boundary

- ODT remains the canonical saved format.
- Widths are hints, not pixel measurements or a full table layout model.
- Missing or invalid width metadata falls back to equal-width behavior.
- External ODT width metadata is accepted only when it is numeric, bounded, shape-matched, and attached to an editable rectangular table.
- No arbitrary CSS, external table themes, merged cells, formulas, spreadsheet import, or rich table layout fidelity is introduced.

## Privacy And Safety

- Width metadata stores only bounded integers and contains no paths, filenames, usernames, hostnames, account identifiers, telemetry identifiers, source names, or document text.
- HTML and print HTML export generate percentages from sanitized integers instead of preserving imported style strings.
- Invalid or mismatched external values are ignored instead of trusted.
- The feature adds no telemetry, network behavior, cloud sync, accounts, remote resource loading, or heavy dependencies.

## Deferred

- Merged cells, formulas, arbitrary CSS, per-side borders, rich table themes, spreadsheet/rich HTML paste, drag resizing, full external table style compatibility, and a full table layout engine.

## Verification

- Independent builder and reviewer agent passes completed before control-tower acceptance.
- `./scripts/verify-local.sh`
- `npm run check`
- `npm run test`
- `cargo test -p word-core table`
- `cargo test -p word-odf table_column`
- `cargo test -p word-export table`
- `cargo test -p nine-hundred-word table`
- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `git diff --check`
