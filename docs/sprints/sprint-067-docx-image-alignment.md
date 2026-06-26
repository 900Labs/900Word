# Sprint 067: DOCX Image Alignment Interoperability

## Goal

Round-trip the existing 900Word image alignment metadata through generated DOCX image paragraphs without claiming floating, wrapping, anchoring, or broad Word media layout fidelity.

## Completed

- Imported simple DOCX paragraph `w:jc` values as image alignment only when the paragraph contains exactly one supported image.
- Supported left, center, and right alignment; justify remains unsupported and imports as inline/default image alignment.
- Kept mixed text/image paragraphs from promoting paragraph alignment into hidden image metadata.
- Exported 900Word-authored left, center, and right image alignment as generated paragraph `w:jc` around supported image blocks.
- Added focused `word-docx` tests for import, mixed-content fallback, generated export XML, and image asset round-trip.

## Boundaries

- ODT remains the canonical saved format.
- DOCX remains conversion-only; opening `.docx` creates an unsaved dirty document.
- Image alignment import is limited to paragraphs that contain exactly one supported image and no surrounding text or page-break content.
- Floating images, text wrapping, anchoring, positioning, justify alignment, arbitrary media layout fidelity, telemetry, network behavior, accounts, cloud sync, plugin behavior, and heavy dependencies remain deferred.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-docx image_alignment`
- `cargo test -p word-docx image_assets`
- `cargo test -p word-docx`
- `cargo clippy -p word-docx -- -D warnings`
- `git diff --check`
- source privacy scan for local path and personal-information patterns over changed docs/source
- `./scripts/verify-local.sh`

## Privacy Notes

- Imported image alignment is normalized from simple local DOCX paragraph markup only.
- Exported image alignment tags are generated from local 900Word image presentation metadata.
- Mixed or rich media layout metadata remains ignored instead of being preserved as hidden metadata.
- The feature does not store or emit local paths, private filenames, usernames, hostnames, source document names, account/cloud identifiers, telemetry identifiers, network state, Word anchoring/wrapping metadata, or document-content logs.
