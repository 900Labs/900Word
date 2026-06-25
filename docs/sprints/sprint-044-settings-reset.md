# Sprint 044: Local Settings Reset MVP

## Status

Complete.

## Goal

Add a narrow offline Settings reset flow that restores sanitized local defaults without exposing app-data paths, private filenames, identity details, document content, recovery locations, telemetry identifiers, network state, or recent paths.

## Shipped

- Added a `reset_settings` Tauri command that uses the existing app-data settings path.
- Chose to rewrite `settings.json` with sanitized defaults instead of deleting it, because the existing atomic private write path is already tested and preserves owner-only permissions where supported.
- Returned the sanitized default `Settings` payload to the frontend after reset, with telemetry forced to `false`.
- Added a Reset Settings button beside Save Settings that updates local UI settings from the returned backend payload.
- Added English and Spanish labels/status text while keeping Arabic as the existing RTL smoke locale with English fallback.
- Covered reset defaults, missing-file reset, sanitized write failure errors, unsafe parent errors, and Unix owner-only rewrite permissions with focused backend tests.

## Compatibility Boundary

- Reset affects only the persisted local desktop settings schema: telemetry-disabled state, language tag, UI locale, high contrast, larger toolbar, reduced motion, low-resource mode, and smart typing toggles.
- Reset does not touch documents, recovery snapshots, recent-document backend state, dictionaries, imports, exports, accounts, cloud sync, telemetry, network behavior, plugin-managed settings, or document metadata.
- Reset errors stay generic and do not include local paths, filenames, usernames, hostnames, account/cloud identity, telemetry identifiers, network state, document text, recent paths, or recovery locations.

## Deferred

- Settings import/export.
- Settings migration/version UI.
- Per-document preferences.
- Cloud sync, accounts, plugin-managed settings, telemetry, and network-backed settings remain out of scope.

## Verification

- `cargo test -p nine-hundred-word settings` - passed.
- `npm --workspace apps/desktop run test -- i18n` - passed.
- `npm run check` - passed.
- `npm run test` - passed.
- `cargo fmt --all -- --check` - passed.
- `git diff --check` - passed.
