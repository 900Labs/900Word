# Sprint 063: DOCX Generic Font-Family Interoperability

## Goal

Add bounded DOCX import/export support for inline font-family values by mapping allowlisted Word run fonts into the existing 900Word generic font-family model without preserving arbitrary source font names.

## Completed

- Imported allowlisted `w:rFonts` values into existing 900Word generic inline font-family IDs: `system-ui`, `serif`, `sans-serif`, and `monospace`.
- Exported valid 900Word-authored generic inline font-family IDs as generated static DOCX `w:rFonts` values.
- Ignored unsupported or arbitrary source font names without persisting them into `word-core`.
- Added focused `word-docx` tests for safe import, unsupported import, started-element hardening, and export/import round-trip behavior.

## Boundaries

- ODT remains the canonical saved format.
- DOCX remains conversion-only; opening `.docx` creates an unsaved dirty document.
- The converter maps common Word fonts to generic local families; it does not claim exact font matching.
- Arbitrary source font names, embedded fonts, theme fonts, inherited character styles, complex script font attributes, exact layout fidelity, telemetry, network behavior, accounts, cloud sync, plugin behavior, and heavy dependencies remain deferred.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-docx inline_formatting`
- `cargo test -p word-docx run_fonts`
- `cargo test -p word-docx`
- `cargo clippy -p word-docx -- -D warnings`
- `./scripts/verify-local.sh`

## Privacy Notes

- Imported DOCX font names are accepted only when they match the allowlist and are immediately normalized to generic local family IDs.
- Unsupported font names are ignored and are not stored, exported, logged, or shown in warnings.
- Exported DOCX font tags are generated from local generic family IDs and do not include source font names.
- The feature does not store or emit local paths, private filenames, usernames, hostnames, source document names, account/cloud identifiers, telemetry identifiers, network state, or document-content logs.
- The feature adds no telemetry, network calls, cloud sync, accounts, remote fetching, plugin behavior, or heavy dependencies.
