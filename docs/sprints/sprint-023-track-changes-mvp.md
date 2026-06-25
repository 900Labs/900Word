# Sprint 023: Track Changes MVP

## Status

Complete.

## Goal

Add lightweight local text-only track changes while keeping `word-core` canonical, preserving existing comments/links/formatting projection, and avoiding collaboration, identity, cloud, telemetry, DOCX parity, or export-fidelity overclaims.

## Shipped

- Added document-level Record changes state to `word-core`.
- Added inline tracked-change metadata for text insertions and selected-text deletions, including safe local change IDs, kind, `Local User` author metadata, and timestamps.
- Added `word-core` commands to accept/reject one change and accept/reject all changes. Accepting deletions removes the deleted text; rejecting insertions removes the inserted text; accepted insertions and rejected deletions keep text and clear change metadata.
- Added a ProseMirror `trackedChange` mark that coexists with comments, links, direct text style, and standard inline formatting.
- Added editor recording helpers for typed text insertion and selected Backspace/Delete behavior while recording is enabled.
- Added a compact desktop review surface with Record changes, change count, individual accept/reject, accept all/reject all, jump-to-change, and a visible privacy warning.
- Added ODT save/reopen support for 900Word-authored text-only tracked changes using `word900` metadata on inline spans plus document-level recording state.

## Compatibility Boundary

- Track changes are text-only for this MVP.
- ODT persistence is for 900Word-authored tracked changes. Metadata uses the existing `word900` namespace for local change ID, kind, author, timestamp, and recording state.
- Pending tracked deletions intentionally keep deleted text in the saved `.odt` package until accepted or rejected.
- TXT, HTML, print HTML, and basic PDF export do not claim track-changes fidelity. Pending inserted/deleted text may appear as normal document text in simple exports.

## Deferred

- Formatting-only changes, table structure changes, image changes, comments-as-changes, page-region changes, DOCX track changes, PDF review metadata, compare/merge, review filters, and multi-author collaboration.
- Full external ODT change-tracking compatibility.
- Rich paste tracking across multi-block paste and every exotic deletion path.

## Verification

- `npm run check` - passed.
- `npm run lint` - passed.
- `npm run test` - passed.
- `cargo fmt --all -- --check` - passed.
- `cargo test --workspace` - passed.
- `git diff --check` - passed.
- `./scripts/verify-public-release.sh` - passed.

## Privacy Notes

- Tracked-change authors default to `Local User`; 900Word does not read OS usernames, hostnames, account identifiers, contacts, local paths, source filenames, or cloud identity.
- Tracked changes can reveal edit history and deleted text in saved files. Accept or reject pending changes before sharing when edit history is sensitive.
- The feature adds no telemetry, accounts, cloud sync, remote lookup, or network calls.
