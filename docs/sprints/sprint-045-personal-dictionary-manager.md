# Sprint 045: Personal Dictionary Manager MVP

## Status

Complete.

## Goal

Add a narrow local personal dictionary manager under Settings so users can inspect normalized personal words for the active spell-check dictionary and remove selected words without adding imports, exports, downloads, cloud sync, accounts, telemetry, remote lookup, plugin-managed dictionaries, or document metadata changes.

## Shipped

- Added `word-spell` helpers to list and remove personal dictionary words through the existing app-data user dictionary boundary.
- Kept personal dictionary words normalized, sorted, bounded, and returned as plain words only.
- Added Tauri commands for listing and removing personal words with generic privacy-safe errors.
- Extended the Settings dictionary manager with a compact personal dictionary section for the active dictionary, including refresh, remove, empty state, and generic unavailable state.
- Removed deleted words from the session-level ignored spelling set and re-ran spell-check so removed words can surface again in the current session.
- Added English and Spanish labels while keeping Arabic as the existing RTL smoke locale with English fallback.

## Compatibility Boundary

- Personal dictionary files remain local app-data files under the same backend-only dictionary boundary used by existing spell-check commands.
- The frontend receives only normalized personal words; it does not receive local paths, personal dictionary filenames, private dictionary filenames, usernames, hostnames, recent paths, recovery locations, account/cloud identity, telemetry identifiers, network state, or dictionary metadata.
- The feature does not add dictionary import/export UI, downloads, remote dictionary lookup, cloud sync, accounts, telemetry, network behavior, plugin-managed dictionaries, or document metadata changes.

## Deferred

- Personal dictionary import/export.
- Bulk selection, search, and richer editing controls.
- Locale-specific casing beyond the existing bounded normalization.
- Full dictionary installation UX and reviewed full-size bundled dictionaries.

## Verification

- `cargo test -p word-spell personal`
- `cargo test -p nine-hundred-word personal_dictionary`
- `cargo test -p nine-hundred-word settings`
- `npm --workspace apps/desktop run test -- i18n`
- `npm --workspace apps/desktop run check`
- `cargo fmt --all -- --check`
- `git diff --check`

## Privacy Notes

- List/remove commands validate the language tag and local dictionary root through existing backend boundaries.
- Errors and UI states are generic and do not include local paths, filenames, usernames, hostnames, document text, recent paths, account/cloud identity, telemetry identifiers, network state, or recovery locations.
- Removing a personal word rewrites the remaining local word list with owner-only file permissions on Unix platforms where supported.
