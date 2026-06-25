# Sprint 042: Dictionary Manager MVP

## Status

Complete.

## Goal

Add a narrow offline dictionary manager under Settings so users can inspect installed local dictionaries and choose the active spell-check dictionary without adding downloads, cloud sync, telemetry, network access, plugin runtime behavior, or new dictionary import workflows.

## Shipped

- Replaced the single Settings dictionary dropdown with a compact dictionary manager section.
- Kept active spell-check dictionary selection on the existing `language_tag` settings field and existing Save Settings flow.
- Added a dictionary list refresh button that reuses the existing `list_dictionaries` command.
- Listed installed bundled/user dictionaries with display name, language tag, source type, license, and generic source labels.
- Added explicit offline/local-only copy and a graceful unavailable-selected-dictionary message for fallback behavior.
- Added English and Spanish labels for the new dictionary manager surface and verified the Arabic RTL smoke locale keeps the standard English fallback behavior.

## Compatibility Boundary

- This sprint does not add dictionary downloads, remote dictionary lookup, cloud sync, accounts, telemetry, plugin runtime behavior, native file-picker install flows, or settings-file format changes beyond the active language tag field later persisted by Sprint 043.
- User dictionaries are still discovered only through the existing local app-data dictionary folder boundary.
- Missing selected dictionaries still fall back to the bundled English bootstrap dictionary during spell checks.

## Deferred

- Dictionary install/import UX.
- Rich personal dictionary word management.
- Full Hunspell affix expansion and reviewed full-size bundled dictionaries.

## Verification

- `npm --workspace apps/desktop run test -- i18n` - required for new i18n labels.
- `npm --workspace apps/desktop run check` - required for the Svelte Settings change.
- `git diff --check` - required for touched source/docs.

## Privacy Notes

- Dictionary manager labels are derived from existing sanitized dictionary metadata plus static generic source labels.
- The UI does not render local paths, private filenames, usernames, hostnames, source document names, dictionary filenames, telemetry identifiers, cloud identity, or network state.
- The feature remains local-only and offline; it does not send document text, dictionary metadata, or personal dictionary words to remote services.
