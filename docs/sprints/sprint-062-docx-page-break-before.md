# Sprint 062: DOCX Paragraph Page-Break-Before Interoperability

## Goal

Add bounded DOCX import support for paragraph-level page-break-before flags by normalizing supported top-level body paragraph flags into the existing local `PageBreak` block model without preserving hidden Word layout metadata.

## Completed

- Imported truthy top-level body paragraph `w:pageBreakBefore` flags as local `PageBreak` blocks before the affected paragraph.
- Ignored falsy `w:pageBreakBefore` values.
- Ignored nested paragraph flags such as table-cell `pageBreakBefore` with generic DOCX page-break degradation warnings.
- Kept DOCX export on the existing generated explicit `w:br w:type="page"` run shape for local `PageBreak` blocks.
- Added focused `word-docx` tests for supported import, falsy values, and nested table-cell degradation.

## Boundaries

- ODT remains the canonical saved format.
- DOCX remains conversion-only; opening `.docx` creates an unsaved dirty document.
- Only truthy top-level body paragraph `w:pageBreakBefore` flags are converted to page-break blocks.
- Nested paragraph flags, hidden Word layout fidelity, deterministic pagination, complex section semantics, telemetry, network behavior, accounts, cloud sync, plugin behavior, and heavy dependencies remain deferred.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-docx page_break_before`
- `cargo test -p word-docx page_break`
- `cargo test -p word-docx`
- `cargo clippy -p word-docx -- -D warnings`
- `./scripts/verify-local.sh`

## Privacy Notes

- Imported `pageBreakBefore` flags are normalized only into local block structure.
- Unsupported nested flags produce generic warnings and do not preserve package entry names, source layout metadata, local paths, private filenames, usernames, hostnames, source document names, account/cloud identifiers, telemetry identifiers, network state, or document-content logs.
- Exported DOCX page breaks are generated from local `word-core` blocks and do not preserve hidden Word paragraph flags.
- The feature adds no telemetry, network calls, cloud sync, accounts, remote fetching, plugin behavior, or heavy dependencies.
