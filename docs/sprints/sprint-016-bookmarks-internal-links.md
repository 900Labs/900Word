# Sprint 016: Bookmarks And Internal Links MVP

## Status

Complete.

## Goal

Add a lightweight, local-first bookmark and internal-link layer for 900Word-authored documents without adding cloud services, telemetry, accounts, remote loads, local path storage, or heavy dependencies.

## Shipped

- Added optional `bookmark_id` fields to durable paragraph and heading blocks in `word-core`.
- Added ProseMirror paragraph/heading `bookmarkId` attrs with strict validation and preservation through projection and sync.
- Added compact toolbar controls to add/remove a bookmark on the selected paragraph or heading.
- Added an internal target dropdown to the existing link popover. It writes safe existing `#bookmark-id` hrefs through the existing link mark path.
- Extended link sanitization to allow only `http`, `https`, `mailto`, and safe local fragments.
- Preserved 900Word-authored bookmarks in ODT with native `text:bookmark` elements inside `text:p` and `text:h`.
- Preserved internal links in ODT as safe `#fragment` text links.
- Stripped unsafe imported bookmark names and unsafe internal hrefs with generic warnings.
- Emitted sanitized HTML element `id` attributes and internal fragment hrefs.
- Preserved bookmark IDs when converting bookmarked paragraphs/headings into list items.

## Deferred

- Automatic bookmark IDs for every heading.
- A full bookmark manager or rename workflow.
- Existing safe internal links from imported or previously edited documents may become stale if users later remove their target bookmark. Rich stale-link inspection and repair is deferred.
- Cross-document links.
- Active PDF link annotations. Basic PDF remains text-oriented.
- Richer external ODT bookmark forms beyond the safe paragraph/heading subset.

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
