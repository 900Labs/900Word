# Sprint 022: Comments MVP

## Status

Complete.

## Goal

Add lightweight local comments for selected text while keeping `word-core` canonical, preserving existing formatting/link/style projection, and avoiding identity, cloud, telemetry, or export-fidelity overclaims.

## Shipped

- Added durable `word-core` comment threads with bounded safe IDs, bounded bodies, privacy-safe `Local User` author defaulting, resolved state, timestamps, and inline `comment_ids` anchors.
- Added comment commands for add, resolve/unresolve, and delete. Delete removes dangling inline anchors, and block edits prune comment metadata when the anchored text is removed.
- Added a ProseMirror `comment` mark and projection tests proving comments round-trip beside bold, links, and direct text style metadata.
- Added editor helpers for adding comments to non-empty selected text, selecting the first anchor for a comment, and removing temporary anchors on failure.
- Added a compact comments sidebar with add, jump, resolve/reopen, and delete controls plus visible highlighted comment markers in the editor.
- Added the Insert Comment shortcut (`Cmd+Option+M` on macOS, `Ctrl+Alt+M` elsewhere) to open the comments panel and focus the comment body.
- Added ODT save/reopen support for 900Word-authored comments using ODF `office:annotation` / `office:annotation-end` around selected text ranges. `word900` metadata records the local comment ID and resolved state.

## Compatibility Boundary

- ODT comments are intended for 900Word-authored annotations. External annotations with unsafe IDs or invalid bodies are ignored with generic warnings instead of being trusted.
- The resolved/unresolved state is stored in the `word900` namespace because ODF annotation compatibility does not provide the full local thread state needed by this MVP.
- TXT, HTML, print HTML, and basic PDF export remain conservative. They render document text and do not claim comment fidelity, active PDF annotations, or DOCX comment interoperability.

## Deferred

- Thread replies, multiple authors, user profile settings, mentions, timestamps editing, filters, and comment search.
- Full external ODT annotation compatibility, DOCX comment import/export, and PDF annotation export.
- Rich anchoring across unsupported external structures, images, page regions, or read-only imported content.

## Verification

- `./scripts/verify-local.sh` - passed, including frontend check/lint/test/build, bundle budget, offline runtime scan, package scan, SBOM generation, performance smoke, Rust fmt/check/test/clippy, public-release scan, cargo audit, cargo deny, and high-severity npm audit gate.
- `npm run check` - passed.
- `npm run lint` - passed.
- `npm run test` - passed.
- `cargo fmt --all -- --check` - passed.
- `cargo check --workspace` - passed.
- `cargo test --workspace` - passed.
- `cargo clippy --workspace -- -D warnings` - passed.
- `git diff --check` - passed.
- `./scripts/verify-public-release.sh` - passed.

## Privacy Notes

- Comment authors default to `Local User`; no usernames, hostnames, account IDs, source filenames, or local paths are generated.
- Comments are stored locally inside the document model and saved `.odt` package only when the user saves the document.
- The feature adds no telemetry, accounts, cloud sync, remote lookup, or network calls.
