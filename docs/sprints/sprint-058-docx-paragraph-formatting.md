# Sprint 058: DOCX Paragraph Formatting Interoperability

## Goal

Add bounded DOCX import/export support for the existing 900Word paragraph formatting model without broadening into full Word styles or layout fidelity.

## Completed

- Imported simple DOCX paragraph `w:jc` alignment into `ParagraphFormat`.
- Imported automatic `w:spacing/@w:line` line spacing into bounded per-mille line spacing.
- Imported bounded spacing before/after, start/end indents, and first-line or hanging indents from DOCX twips into 900Word millimeter values.
- Exported valid 900Word-authored paragraph formatting as generated DOCX `w:pPr` spacing, indent, and alignment tags.
- Added round-trip, import, and unsupported-value tests in `word-docx`.

## Boundaries

- ODT remains the canonical saved format.
- DOCX remains conversion-only; opening `.docx` creates an unsaved dirty document.
- Unsupported line rules, negative spacing, extreme values, inherited Word styles, tabs, paragraph borders, paragraph shading, outline/keep flags, complex section layout, deterministic pagination, and arbitrary paragraph settings remain deferred.
- Heading blocks do not gain a new paragraph-format storage model; headings keep the existing `word-core` shape.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-docx`
- `cargo clippy -p word-docx -- -D warnings`

## Privacy Notes

- The DOCX converter writes only generated alignment, spacing, and indent values from the local `word-core` model.
- Imported DOCX paragraph settings are normalized only into bounded numeric presentation values.
- The feature does not store or emit local paths, private filenames, usernames, hostnames, source document names, account/cloud identifiers, telemetry identifiers, network state, or document-content logs.
- The feature adds no telemetry, network calls, cloud sync, accounts, remote fetching, plugin behavior, or heavy dependencies.
