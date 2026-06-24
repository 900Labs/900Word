# Sprint 011: Structure, Navigation, And Links

Status: complete.

## Scope

This sprint starts the practical document-structure and media/link slice from the upgrade plan while keeping 900Word lightweight, offline, ODT-native, and privacy-safe.

## Completed In This Slice

- Added a live navigator sidebar generated from non-empty Heading 1, Heading 2, and Heading 3 blocks.
- Added click-to-jump navigation from the sidebar into the matching projected editor block.
- Added tested projection helpers for deriving document outlines from both saved `word-core` state and live editor changes.
- Added insert/edit hyperlink and remove-link toolbar workflows with a compact URL popover and Cmd/Ctrl+K shortcut.
- Reused the existing safe-link boundary so editor-authored links are limited to `http`, `https`, and `mailto` targets.
- Preserved the existing ODT/HTML/export-safe link path instead of adding new link storage or network behavior.

## Current Limits

- Bookmarks and internal link-to-bookmark targets remain deferred. The current safe-link sanitizer accepts only external `http`, `https`, and `mailto` links.
- Tables and images remain durable in `word-core`, ODT, TXT, HTML, and basic PDF text/placeholder export paths, but they are still not desktop-editable through the ProseMirror projection.
- Header, footer, page-number, and date-field modeling remain deferred because they need a durable model, ODT representation, and export behavior together.
- Image insertion from local files remains Sprint 012 work. ODT package persistence for embedded image bytes already exists, but the desktop app still needs a safe picker/copy/compression flow.

## Verification

- `npm --workspace apps/desktop run test -- documentProjection editor`
- `npm run test`
- `npm run lint`
- `npm run check`
- `npm run build`
- `cargo clippy --workspace -- -D warnings`
- `./scripts/verify-local.sh`
- Independent reviewer pass completed after the link-boundary, toolbar-state, and responsive-popover fixes.

## Privacy Notes

- No telemetry, cloud calls, accounts, remote fetches, screenshots, local paths, private filenames, or real documents were added.
- The navigator derives headings only from the in-memory document model.
- The link editor validates URLs locally and does not open, prefetch, or contact link targets.
