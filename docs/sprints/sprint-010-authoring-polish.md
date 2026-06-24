# Sprint 010: Authoring Polish

Status: complete.

## Scope

This sprint closes the practical Sprint 1 authoring gaps left after the authoring foundation. It keeps the app lightweight, local-first, offline, and ODT-native while making the existing editor controls behave more like a small word processor.

## Completed In This Slice

- Added selection-derived toolbar state for paragraph style, paragraph formatting, inline formatting, list type, list level, and selection word count.
- Added a durable `update_style` document command and UI command that updates the selected paragraph style from the selected paragraph's direct paragraph formatting.
- Preserved generated paragraph style properties through ODT write/read for 900Word-authored style updates.
- Added list-aware Enter handling for continuing non-empty list items and exiting lists from an empty item.
- Added plain-text paste handling for basic newline paragraphs and simple bullet/numbered list text.
- Replaced status-only explicit spell checks with Hunspell-backed misspelling decorations, suggestions, ignore-once, ignore-all, and local personal dictionary additions.
- Expanded the footer stats panel with selection word count, characters without spaces, paragraph count, and estimated reading time.
- Added active toolbar button state and filled missing open/save-as shortcut coverage in the app shell.

## Current Limits

- Update-style-from-selection intentionally supports paragraph style properties only. Full arbitrary style editing, character style editing, and a persistent style management panel remain deferred.
- Spell suggestions are bounded and dictionary-local. They are useful for small bundled or user dictionaries, but they are not a full Hunspell affix suggestion engine.
- Personal dictionary additions are local app data and supplement checks; the UI does not yet expose a full dictionary manager.
- List paste support targets common plain-text list markers and newline paragraphs. Rich clipboard HTML/table/list import remains deferred.

## Verification

- `cargo fmt`
- `npm run lint`
- `npm run check`
- `npm run test`
- `cargo test -p word-core -p word-spell -p word-odf`

## Privacy Notes

- No telemetry, cloud calls, accounts, private endpoints, local paths, screenshots, private filenames, or real documents were added.
- Spell checking and personal dictionary writes stay local.
- New tests use synthetic words and generated in-memory ODT content only.
