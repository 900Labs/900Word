# Sprint 026: Autocorrect And Smart Typing MVP

## Status

Complete.

## Goal

Add a bounded local-first autocorrect and smart typing MVP for user-typed editor input while keeping 900Word lightweight, offline, ODT-native, deterministic, and free of cloud, account, telemetry, network, AI, or heavyweight dependency additions.

## Shipped

- Added disabled-by-default local Smart typing settings for sentence capitalization, smart quotes, double-hyphen em dashes, allowlisted typo replacements, and simple list triggers.
- Added deterministic typed-input transforms in the ProseMirror editor for collapsed user input only.
- Added sentence capitalization at paragraph start and after sentence-ending punctuation plus following whitespace.
- Added straight quote conversion to directional smart quote characters.
- Added double-hyphen conversion to an em dash outside URL-like tokens.
- Added a small bundled typo replacement map for common English typos.
- Added simple `- ` and `1. ` list triggers at the start of an otherwise empty top-level paragraph.
- Added Settings-view controls and UI strings for the new smart typing options.
- Kept smart typing out of `word-core`, ODT import/export metadata, document commands, network access, telemetry, accounts, and persistent document state.

## Compatibility Boundary

- Smart typing applies only to future user typing in the desktop editor. It does not clean up imported documents or rewrite existing large selections.
- Transforms are bounded to small collapsed input events and skip URL-like tokens for typo and dash behavior.
- The typo replacement map is intentionally small, local, and allowlisted.
- List triggers are limited to top-level empty paragraphs that contain only the typed trigger marker.
- The current settings path follows the existing local settings model and does not introduce a new settings file format.

## Deferred

- Persistent settings storage across app restarts.
- Locale-specific smart quote rules and larger dictionaries.
- Rich autocorrect management UI, custom replacement entries, undo grouping polish, and per-document language behavior.
- Smart typing while track-changes recording is active.
- Imported-document cleanup, paste-wide autocorrect, and transforms across large ranges.

## Verification

- `npm run check --workspace apps/desktop` - passed.
- `npm run lint --workspace apps/desktop` - passed.
- `npm run test --workspace apps/desktop -- smartTyping editor i18n` - passed.
- `cargo test -p nine-hundred-word settings_never_enable_telemetry` - passed.
- `cargo fmt --all -- --check` - passed.
- `git diff --check` - passed.

## Privacy Notes

- The feature is local typed-input behavior only.
- The bundled typo map is static source data and no document text leaves the editor.
- Smart typing does not add telemetry, accounts, cloud sync, remote lookup, AI services, document-content logging, local path access, hostname access, username access, or new document metadata.
