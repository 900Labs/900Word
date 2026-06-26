# Sprint 068: DOCX Image Alt-Text Interoperability

## Goal

Round-trip existing 900Word image alt text through the bounded DOCX conversion layer without preserving source image names, paths, relationship targets, or richer media metadata.

## Completed

- Exported local `ImageBlock.alt_text` as generated DOCX drawing `descr` attributes on both `wp:docPr` and `pic:cNvPr`.
- Imported supported `descr`/`title` metadata from `wp:docPr` and `pic:cNvPr` into existing 900Word image alt text.
- Kept DOCX image export media part names and relationship IDs generated from local state rather than source filenames or paths.
- Added focused `word-docx` tests for picture metadata import, escaped export attributes, and DOCX image round-trip behavior.

## Boundaries

- DOCX remains conversion-only; opening `.docx` creates an unsaved dirty document.
- Image alt text is document content and may be visible to recipients of exported files.
- Imported alt text is bounded and normalized into the existing local image alt-text field.
- Source image names, relationship target names, package entry names, local paths, usernames, hostnames, linked/remote images, rich media metadata, arbitrary media layout fidelity, telemetry, network behavior, accounts, cloud sync, plugin behavior, and heavy dependencies remain deferred.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-docx image_alt`
- `cargo test -p word-docx image_assets`
- `cargo test -p word-docx`
- `cargo clippy -p word-docx -- -D warnings`
- `git diff --check`
- changed-file privacy scan for local path and personal-info patterns
- `./scripts/verify-local.sh`

## Privacy Notes

- Exported DOCX alt text is generated only from local document alt text.
- Imported DOCX alt text is accepted only from supported drawing metadata attributes and is not treated as source identity metadata.
- The feature does not store or emit local paths, private filenames, usernames, hostnames, source document names, source image names, relationship target names, account/cloud identifiers, telemetry identifiers, network state, or document-content logs.
