# Sprint 060: DOCX Page Setup Interoperability

## Goal

Add bounded DOCX import/export support for the existing 900Word page setup model without broadening into full Word section layout or pagination fidelity.

## Completed

- Imported simple complete body-level section DOCX `w:pgSz` width and height values into `PageSetup`.
- Imported simple complete body-level section DOCX `w:pgMar` top, right, bottom, and left margins into `PageSetup`.
- Normalized DOCX twip values to millimeters and accepted them only when the final `PageSetup` validates.
- Added a generic warning when DOCX page setup is present but cannot be safely imported.
- Exported valid 900Word-authored page setup as generated DOCX `w:pgSz` and `w:pgMar` tags.
- Added import, unsupported-value, and export/import round-trip tests in `word-docx`.

## Boundaries

- ODT remains the canonical saved format.
- DOCX remains conversion-only; opening `.docx` creates an unsaved dirty document.
- Only the body-level section page setup represented by the single current 900Word section model is imported/exported.
- Header/footer distances, gutter, columns, orientation metadata beyond explicit width/height, multi-section layout, page borders, paper trays, deterministic pagination, and full Word section layout fidelity remain deferred.
- Partial, invalid, or out-of-model page setup values are ignored with a generic warning rather than stored as hidden metadata.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-docx`
- `cargo clippy -p word-docx -- -D warnings`

## Privacy Notes

- Imported DOCX page setup values are normalized only into bounded numeric local page metadata.
- Exported DOCX page setup values are generated from local `word-core` content and do not include source section metadata that 900Word does not model.
- The feature does not store or emit local paths, private filenames, usernames, hostnames, source document names, account/cloud identifiers, telemetry identifiers, network state, or document-content logs.
- The feature adds no telemetry, network calls, cloud sync, accounts, remote fetching, plugin behavior, or heavy dependencies.
