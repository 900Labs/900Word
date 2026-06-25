# Sprint 046: Local Hunspell Dictionary Install MVP

## Status

Complete.

## Goal

Add an offline-only Settings flow for installing a local Hunspell `.aff`/`.dic` dictionary pair into 900Word app data, then refresh and select it through the existing dictionary manager without adding downloads, remote lookup, cloud sync, accounts, telemetry, plugin dictionaries, or document metadata changes.

## Shipped

- Added a `word-spell` install helper that validates language tags, source file shape, extensions, size limits, symlinks, UTF-8 readability, source/target overlap, and copied dictionary parseability before exposing installed files.
- Copied accepted dictionary pairs into the app-data dictionary root using normalized `<language-tag>.aff` and `<language-tag>.dic` filenames, owner-only permissions where supported, and temp-file cleanup on failures.
- Added a Tauri `install_user_dictionary` command that receives native-dialog path selections, reuses the existing user dictionary root, and maps failures to generic privacy-safe categories.
- Extended the Settings dictionary manager with compact local install controls, native `.aff`/`.dic` pickers, generic selected-state labels, install status messages, automatic dictionary refresh, active language selection, and personal dictionary refresh.
- Added English and Spanish labels while keeping Arabic on the existing English fallback path.

## Compatibility Boundary

- Installed dictionaries remain local app-data files under the existing backend-only dictionary boundary.
- Source paths from native dialogs are transient frontend state passed to Rust only for the install command.
- The UI never renders selected local paths or filenames; it shows only generic AFF/DIC selected states.
- The install flow does not change saved documents, document metadata, ODT/TXT/HTML/print HTML/PDF/DOCX output, recent documents, recovery state, accounts, telemetry, cloud sync, plugin runtime behavior, network behavior, or remote dictionary behavior.

## Deferred

- Dictionary downloads or remote dictionary discovery.
- Plugin-managed dictionaries.
- Dictionary import/export packaging beyond local Hunspell pair copy.
- Full Hunspell affix expansion and reviewed full-size bundled dictionaries.
- Rich validation diagnostics that expose local file details.

## Verification

- `cargo test -p word-spell dictionary`
- `cargo test -p nine-hundred-word dictionary`
- `npm --workspace apps/desktop run test -- i18n`
- `npm --workspace apps/desktop run check`
- `cargo fmt --all -- --check`
- `git diff --check`

## Privacy Notes

- Install errors returned through Tauri are generic and do not include local source paths, app-data paths, filenames, usernames, hostnames, dictionary stems, document text, account/cloud identity, telemetry identifiers, network state, or recovery paths.
- Installed dictionary files use normalized language filenames in app data and owner-only permissions on Unix platforms where supported.
- Temp install files are cleaned up on validation or copy failures so a failed install does not leave a visible half-installed dictionary.
