# Sprint 014: Local Image Insertion

Status: complete.

## Scope

This sprint adds a bounded MVP for inserting local images into the lightweight word processor. It keeps 900Word local-first, keeps ODT as the native saved format, keeps `word-core` as durable truth, and does not add telemetry, accounts, cloud sync, remote image fetching, or network behavior.

## Completed In This Slice

- Added a desktop Insert Image command backed by the native open dialog with PNG, JPEG, GIF, and WebP filters.
- Added a Rust IPC command that receives the selected local path, validates it as untrusted input, copies accepted bytes into `document.assets`, and inserts an `ImageBlock`.
- Enforced traversal rejection, supported-extension checks, regular-file checks, 8 MiB maximum image size, and magic-byte validation for PNG, JPEG, GIF, and WebP.
- Generated generic asset names in the form `image-<uuid>.<ext>` and left `AssetRef.original_name` empty for local imports so source paths and private source filenames are not serialized.
- Inserted near the current top-level editor selection when available, with backend append behavior when the requested target is missing or out of range.
- Added a ProseMirror image atom projection that renders embedded image bytes as offline data URLs and round-trips back to `ImageBlock` so later text edits do not silently drop images.
- Updated HTML and print HTML export to emit allowlisted in-document image assets as embedded data URLs only.
- Validated image bytes again during HTML export before emitting data URLs so mislabeled asset metadata remains a safe placeholder.
- Made image import a single undoable mutation so undo removes both the visible image block and the embedded asset bytes.
- Preserved ODT image round-trip behavior through existing embedded asset package support and focused tests.

## Current Limits

- Image blocks are non-editable atoms in the body editor.
- Alt text defaults to `Image`; dedicated alt text editing is deferred to the next image-polish slice.
- Image resizing, cropping, captions beyond generic alt display, drag/drop insertion, and table/header/footer image editing are deferred.
- Basic PDF export remains text-oriented and includes image alt/caption text only; raster PDF embedding is deferred.

## Privacy Notes

- The selected source path is used only as Rust IPC input and is not stored in document state after import.
- Source filenames are not copied into `AssetRef.original_name`, status text, exports, docs, fixtures, or logs.
- Unsupported, mismatched, traversal-shaped, empty, oversized, and unsafe image files fail with generic errors.
- HTML export emits only embedded `data:` image URLs for allowlisted in-document assets and never remote or `file:` image URLs.

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
