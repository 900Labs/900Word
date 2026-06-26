# Sprint 066: DOCX Generated Image Caption Interoperability

## Goal

Round-trip 900Word-authored image captions through DOCX by using a generated visible paragraph style without claiming Word-native caption field or rich media layout fidelity.

## Completed

- Exported non-empty local image captions as visible `Word900ImageCaption` paragraphs immediately after supported DOCX image blocks.
- Added a generated DOCX paragraph style for those image caption paragraphs.
- Imported plain `Word900ImageCaption` paragraphs back into the preceding supported image block caption metadata.
- Kept unstyled, orphaned, linked, rich, or otherwise unsupported caption-like paragraphs as visible document paragraphs.
- Added focused `word-docx` tests for generated caption import, image export/import round-trip, and fallback behavior.

## Boundaries

- ODT remains the canonical saved format.
- DOCX remains conversion-only; opening `.docx` creates an unsaved dirty document.
- Generated image-caption import is limited to plain `Word900ImageCaption` paragraphs immediately following a supported image.
- Word-native caption fields, sequence fields, cross references, rich caption formatting, arbitrary image layout fidelity, caption numbering, telemetry, network behavior, accounts, cloud sync, plugin behavior, and heavy dependencies remain deferred.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-docx image_caption`
- `cargo test -p word-docx image_assets`
- `cargo test -p word-docx`
- `cargo clippy -p word-docx -- -D warnings`
- `./scripts/verify-local.sh`

## Privacy Notes

- Exported caption paragraphs are generated from local document caption text only.
- Imported generated captions are accepted only as visible plain paragraph text and are not treated as source identity metadata.
- Unsupported caption-like paragraphs remain visible document text rather than hidden metadata.
- The feature does not store or emit local paths, private filenames, usernames, hostnames, source document names, account/cloud identifiers, telemetry identifiers, network state, Word-native caption fields, or document-content logs.
- The feature adds no telemetry, network calls, cloud sync, accounts, remote fetching, plugin behavior, or heavy dependencies.
