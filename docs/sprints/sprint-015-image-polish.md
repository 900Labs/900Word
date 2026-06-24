# Sprint 015: Image Polish MVP

Status: complete.

## Scope

This sprint makes Sprint 014 images usable in the lightweight word processor without widening the architecture. `word-core` remains durable truth, ProseMirror remains the editor projection, ODT remains the native saved format, and the slice adds no telemetry, accounts, cloud sync, remote image loading, or heavy image-editing framework.

## Completed In This Slice

- Added durable `ImagePresentation` metadata to `word-core` for alignment, scale percentage, and caption text while keeping editable alt text on `ImageBlock`.
- Kept old image blocks backward-compatible by defaulting missing presentation metadata to inline alignment, 100% scale, and no caption.
- Added compact desktop controls that appear only when an image atom is selected: alt text, caption, alignment, and scale.
- Extended the ProseMirror image atom schema and projection so image metadata survives normal text sync.
- Rendered image preview alignment, scale, caption, and alt text in the editor surface.
- Preserved 900Word-authored image metadata through ODT save/reopen using bounded `word900` metadata attributes on image frames.
- Updated sanitized HTML and print HTML export to emit escaped captions, alt text, alignment, and scale with allowlisted embedded image data URLs.
- Kept TXT and basic PDF export text-oriented while including image alt/caption text in the document text path.
- Added focused tests in `word-core`, `word-odf`, `word-export`, ProseMirror schema/projection, and editor image-attr transactions.

## Current Limits

- Images remain ProseMirror atom blocks; captions are edited in the contextual controls, not directly inside the page surface.
- Scale is stored as a bounded percentage. Native pixel dimensions, crop rectangles, drag resize handles, rotation, and aspect-ratio editing remain deferred.
- ODT persistence for image presentation metadata is reliable for 900Word-authored files through the `word900` namespace, but broad external ODT image layout fidelity is not claimed.
- Basic PDF export still does not embed raster image bytes.
- Header/footer/table-cell image editing remains outside this MVP.

## Privacy Notes

- No source local paths or source filenames are added to image metadata.
- The metadata fields are user-authored document content/presentation values only.
- HTML and print HTML continue to emit only embedded `data:` URLs for allowlisted in-document image assets.
- No new network, telemetry, document logging, account, or cloud behavior was introduced.

## Verification

- `npm run check`
- `npm run lint`
- `npm run test`
- `npm run build`
- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `./scripts/verify-public-release.sh`
- `./scripts/verify-local.sh`
- `git diff --check`
- Touched source/docs privacy scan for local paths, hostnames, and personal names
