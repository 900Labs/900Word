# Sprint 043: Persistent Settings MVP

## Status

Complete.

## Goal

Persist a narrow, sanitized local settings file across app launches while keeping 900Word offline, local-first, telemetry-free, and free of document path or identity storage.

## Shipped

- Added app-data `settings.json` persistence for the existing desktop settings schema.
- Sanitized settings before save and after load.
- Forced `telemetry_enabled` to `false` on every save/load path.
- Preserved language normalization from underscores to hyphens, rejected unsafe/path-shaped language tags, and kept UI locale on the existing allowlist.
- Wrote settings atomically through the existing private atomic-write helper and owner-only directory/file permissions on Unix where supported.
- Loaded sanitized defaults when settings are missing, malformed, unreadable, symlinked, oversized, or otherwise unsafe.
- Kept settings save errors generic so app-data paths and private filenames are not exposed.

## Compatibility Boundary

- Persisted settings include only telemetry-disabled state, language tag, UI locale, high contrast, larger toolbar, reduced motion, low-resource mode, and smart typing toggles.
- The settings file does not store document paths, recent paths, document text, filenames, usernames, hostnames, account/cloud identity, telemetry identifiers, network state, plugin runtime state, import/export metadata, or recovery locations.
- This sprint does not add cloud sync, accounts, telemetry, network behavior, plugin runtime behavior, document metadata, import/export behavior, document-path persistence, recent-document persistence, or UI behavior changes.

## Deferred

- Settings migration/version UI.
- User-visible settings reset/import/export flows.
- Per-document preferences.
- Cloud sync, accounts, plugin-managed settings, telemetry, and network-backed settings remain out of scope.

## Verification

- `cargo test -p nine-hundred-word settings` - passed.
- `cargo test -p nine-hundred-word` - passed.
- `cargo test --workspace` - passed.
- `cargo fmt --all -- --check` - passed.
- `git diff --check` - passed.
