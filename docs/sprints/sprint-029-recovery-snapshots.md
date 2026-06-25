# Sprint 029: Recovery Snapshots MVP

## Status

Complete.

## Goal

Harden local crash recovery by replacing single-file autosave overwrites with bounded versioned recovery snapshots, while keeping recovery local, private, and safe from accidental save-path adoption.

## Shipped

- Autosave now writes versioned recovery snapshots with validated opaque tokens.
- Recovery summaries stay generic: token, label, byte length, and modified timestamp only.
- Recovery labels remain generic and do not include local paths, filenames, usernames, hostnames, source document names, or recovery locations.
- Recovery writes keep owner-only directory and file permissions on Unix.
- Symlinked recovery entries are ignored during listing and rejected during recovery open.
- Recovery open continues to load the draft as dirty and unsaved, with no recovery file adopted as the save path.
- Recovery discard is scoped to the selected validated recovery token and rejects traversal-shaped or plain path input.
- Retention is bounded to 3 snapshots per document and 20 snapshots overall.
- Existing single-file recovery tokens remain valid for listing, recovering, and discarding.

## Compatibility Boundary

- New autosaves write only the versioned snapshot token shape.
- Legacy single-file recovery tokens are not eagerly migrated or renamed.
- Legacy recovery entries can still be listed, recovered, discarded, and counted by the same retention rules.
- If retention pruning runs after new autosaves, older legacy entries may be removed when they are outside the 3-per-document or 20-overall caps.
- Recovery snapshots are local ODT packages and are not encrypted.

## Deferred

- Recovery encryption and key handling.
- Automatic crash hooks, periodic background autosave, close prompts, and startup recovery prompts beyond the current manual autosave and recovery list workflow.
- Rich recovery preview, diffing, merge, and restore history UI.
- User-configurable recovery retention limits.

## Verification

- `cargo test -p nine-hundred-word recovery` - required for recovery token, retention, permissions, symlink rejection, safe-open, legacy compatibility, and discard-scope coverage.
- `cargo test -p nine-hundred-word settings_never_enable_telemetry` - required to re-check the local-first settings guard.
- `cargo fmt --all -- --check` - required for Rust formatting.
- `git diff --check` - required for whitespace safety.
- Touched-file privacy scan for local paths, hostnames, private filenames, usernames, recovery locations, document text, and source image filenames - required for this sprint.

## Privacy Notes

- Recovery tokens are opaque validated values only.
- Recovery labels and summaries do not include local paths, filenames, usernames, hostnames, source document names, recovery locations, or document text.
- The sprint adds no cloud sync, accounts, telemetry, network behavior, AI services, document-content logging, encryption claims, import/export changes, or document metadata.
