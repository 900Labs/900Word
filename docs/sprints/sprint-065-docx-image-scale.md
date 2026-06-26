# Sprint 065: DOCX Image Scale Interoperability

## Goal

Add bounded DOCX import/export support for image scale metadata by mapping generated square `wp:extent` values into the existing 900Word `ImagePresentation.scale_percent` field without claiming arbitrary Word image layout fidelity.

## Completed

- Imported square DOCX `wp:extent` dimensions into existing 900Word image scale percentages when they normalize to the bounded local image scale range.
- Exported valid 900Word-authored image scale metadata as generated square `wp:extent` and DrawingML `a:ext` values.
- Ignored unsupported, non-square, missing, invalid, too-small, and too-large extent values by keeping the default local image scale.
- Skipped nested content inside started `wp:extent` elements so malformed extent markup cannot affect image relationship parsing.
- Added focused `word-docx` tests for safe import, unsupported import, started-element hardening, and image export/import round-trip behavior.

## Boundaries

- ODT remains the canonical saved format.
- DOCX remains conversion-only; opening `.docx` creates an unsaved dirty document.
- Image scale import is a bounded compatibility bridge for the generated 900Word square extent shape, not a full Word image sizing model.
- Arbitrary image sizing, aspect-ratio changes, cropping, rotation, compression/downsampling metadata, linked or remote images, broad media layout fidelity, telemetry, network behavior, accounts, cloud sync, plugin behavior, and heavy dependencies remain deferred.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-docx image_extent`
- `cargo test -p word-docx image_assets`
- `cargo test -p word-docx`
- `cargo clippy -p word-docx -- -D warnings`
- `./scripts/verify-local.sh`

## Privacy Notes

- Imported DOCX image extents are accepted only as bounded numeric scale metadata and do not preserve source filenames, relationship target names, paths, usernames, hostnames, or package entry names.
- Unsupported image extent metadata is ignored rather than logged or surfaced to the frontend.
- Exported DOCX image extent values are generated from local 900Word image scale metadata only.
- The feature does not store or emit local paths, private filenames, usernames, hostnames, source document names, account/cloud identifiers, telemetry identifiers, network state, or document-content logs.
- The feature adds no telemetry, network calls, cloud sync, accounts, remote fetching, plugin behavior, or heavy dependencies.
