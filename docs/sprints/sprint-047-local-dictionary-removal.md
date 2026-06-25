# Sprint 047: Local User Dictionary Removal MVP

## Status

Complete.

## Goal

Add an offline-only Settings flow for removing installed local user Hunspell dictionaries from 900Word app data without removing bundled dictionaries, exposing local paths or filenames, contacting network services, adding accounts or telemetry, touching plugin dictionaries, or changing document metadata.

## Shipped

- Added a `word-spell` removal helper that validates the language tag and removes only regular local user Hunspell `.aff`/`.dic` target files under the app-data dictionary root.
- Removed both normalized `<language-tag>.aff`/`.dic` files and the existing underscore alias pair when present, matching the local loader's user dictionary lookup boundary.
- Kept removal idempotent for missing or partial pairs; incomplete user dictionaries are not listed because listing still requires both regular files.
- Ignored symlink and non-regular targets without following them, so removal never traverses outside the dictionary root.
- Added a Tauri `remove_user_dictionary` command that receives only `language_tag`, reuses the existing user dictionary root, and maps failures to generic privacy-safe strings.
- Added compact Settings remove actions for installed user dictionaries only, with bundled dictionaries hidden from removal, dictionary refresh after removal, active-dictionary fallback, personal word refresh, and spell-check refresh.
- Added English and Spanish labels while keeping Arabic on the existing English fallback path.

## Compatibility Boundary

- Removal affects only user-installed Hunspell pair files in the backend-owned app-data dictionary root.
- Bundled dictionaries remain available, including when a user-installed `en-US` override is removed.
- Personal dictionary word lists are intentionally left in place for a future reinstall of the same language.
- The UI and IPC do not accept arbitrary deletion paths and do not render local paths, filenames, app-data locations, usernames, hostnames, account/cloud identity, telemetry identifiers, network state, recovery paths, or document text.
- The removal flow does not change saved documents, document metadata, ODT/TXT/HTML/print HTML/PDF/DOCX output, recent documents, recovery state, accounts, telemetry, cloud sync, plugin runtime behavior, network behavior, or remote dictionary behavior.

## Deferred

- Deleting personal dictionary word lists together with dictionary removal.
- Confirmation dialogs and richer dictionary management history.
- Dictionary import/export packaging, downloads, remote lookup, and plugin-managed dictionary removal.
- Rich removal diagnostics that expose local file details.

## Verification

- `cargo test -p word-spell dictionary`
- `cargo test -p nine-hundred-word dictionary`
- `npm --workspace apps/desktop run test -- i18n`
- `npm --workspace apps/desktop run check`
- `cargo fmt --all -- --check`
- `git diff --check`

## Privacy Notes

- Removal errors returned through Tauri are generic and do not include local paths, filenames, dictionary stems, usernames, hostnames, document text, account/cloud identity, telemetry identifiers, network state, or recovery paths.
- The backend computes removal targets from a validated language tag and the existing app-data dictionary root; the frontend never sends file paths for removal.
- Symlink and non-regular targets are ignored rather than followed, and incomplete pairs are hidden from the installed user dictionary list.
