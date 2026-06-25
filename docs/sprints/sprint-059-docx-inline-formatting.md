# Sprint 059: DOCX Inline Formatting Interoperability

## Goal

Add bounded DOCX import/export support for the existing 900Word inline text formatting model without broadening into full Word character styles, theme styling, or font fidelity.

## Completed

- Imported simple DOCX run strikethrough from `w:strike` and `w:dstrike`.
- Imported DOCX `w:vertAlign` superscript and subscript into existing inline marks.
- Imported exact supported menu font sizes from DOCX half-point `w:sz` values.
- Imported direct theme-free six-digit DOCX `w:color` values as safe 900Word text colors.
- Imported DOCX `w:highlight` values only when they map to the safe 900Word highlight palette.
- Exported valid 900Word-authored inline formatting as generated DOCX `w:rPr` tags for the same bounded subset.
- Preserved parsed inline styles through the body run flush path used for paragraphs, list items, table cells, revisions, and note-reference fallback text.
- Added import, unsupported-value, and export/import round-trip tests in `word-docx`.

## Boundaries

- ODT remains the canonical saved format.
- DOCX remains conversion-only; opening `.docx` creates an unsaved dirty document.
- Word font family inheritance, arbitrary font names, character styles, theme colors, automatic colors, arbitrary highlight names, complex script font variants, broad run-style inheritance, and full Word style fidelity remain deferred.
- Unsupported or out-of-model run properties are ignored rather than stored as hidden metadata.

## Verification

- `cargo fmt --all`
- `cargo test -p word-docx`

## Privacy Notes

- Imported DOCX run properties are normalized only into existing bounded local inline marks and style values.
- Exported DOCX run properties are generated from local `word-core` content and do not include source formatting metadata that 900Word does not model.
- The feature does not store or emit local paths, private filenames, usernames, hostnames, source document names, account/cloud identifiers, telemetry identifiers, network state, or document-content logs.
- The feature adds no telemetry, network calls, cloud sync, accounts, remote fetching, plugin behavior, or heavy dependencies.
