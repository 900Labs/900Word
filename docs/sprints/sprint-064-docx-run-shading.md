# Sprint 064: DOCX Run Shading Interoperability

## Goal

Add bounded DOCX import support for direct run shading by mapping safe visible `w:shd` fills into the existing 900Word highlight palette without preserving arbitrary Word style metadata.

## Completed

- Imported direct DOCX run `w:shd` fills into existing 900Word inline highlight colors when the fill maps to the safe local palette.
- Accepted only no-pattern `clear` or `solid` shading values, plus omitted `w:val` values that behave as direct fills.
- Treated run shading as a fallback so explicit DOCX `w:highlight` values remain preferred regardless of XML order.
- Ignored `nil`, unsupported pattern fills, theme fills, arbitrary colors, and out-of-palette fills.
- Kept DOCX export on the existing generated `w:highlight` tags for local 900Word-authored highlights.
- Added focused `word-docx` tests for safe import, unsupported import, and started-element hardening.

## Boundaries

- ODT remains the canonical saved format.
- DOCX remains conversion-only; opening `.docx` creates an unsaved dirty document.
- Run shading import is a compatibility bridge into the local highlight model, not a full Word shading or character-style model.
- Pattern fills, theme fills, arbitrary colors, inherited character styles, rich Word style fidelity, exact layout fidelity, telemetry, network behavior, accounts, cloud sync, plugin behavior, and heavy dependencies remain deferred.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-docx run_shading`
- `cargo test -p word-docx inline_formatting`
- `cargo test -p word-docx`
- `cargo clippy -p word-docx -- -D warnings`
- `./scripts/verify-local.sh`

## Privacy Notes

- Imported DOCX run shading is accepted only when it maps to the safe local highlight palette and is immediately normalized to a local color value.
- Pattern fills, theme fills, arbitrary colors, and unsupported shading metadata are ignored and are not stored, exported, logged, or shown in warnings.
- Exported DOCX highlight tags are generated from local 900Word highlight values and do not preserve source shading form.
- The feature does not store or emit local paths, private filenames, usernames, hostnames, source document names, account/cloud identifiers, telemetry identifiers, network state, or document-content logs.
- The feature adds no telemetry, network calls, cloud sync, accounts, remote fetching, plugin behavior, or heavy dependencies.
