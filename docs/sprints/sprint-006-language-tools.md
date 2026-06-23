# Sprint 006: Language Tools

Status: complete.

## Scope

- Add a Hunspell-shaped word-list spell-check boundary.
- Bundle a generated minimal English bootstrap dictionary with license inventory.
- Support user-provided dictionaries from `{APP_DATA_DIR}/dictionaries`.
- Add missing-dictionary fallback behavior.
- Add an initial Svelte UI localization framework.

## Delivered

- `word-spell` now loads UTF-8 Hunspell-shaped `.aff`/`.dic` word-list pairs, lists bundled dictionaries, lists user dictionaries, and loads user dictionaries through a constrained folder boundary.
- A generated minimal `en-US` dictionary is tracked under `crates/word-spell/dictionaries/en_US/` with GPL-3.0-or-later license text.
- Tauri spell-check commands use `{APP_DATA_DIR}/dictionaries`, create it with owner-only permissions on Unix, and fall back to bundled `en-US` when the selected dictionary is missing.
- Dictionary metadata exposed to the frontend includes language tag, display name, source type, and license label, not local paths.
- `apps/desktop/src/lib/i18n.ts` provides English source strings, initial Spanish translations, interpolation, and an RTL smoke locale direction.

## Validation

- `npm run check`
- `cargo test -p word-spell`
- `cargo test -p nine-hundred-word`

## Follow-Ups

- Add full Hunspell affix expansion before claiming broad Hunspell compatibility.
- Replace the generated bootstrap dictionary with reviewed full dictionaries only after license inventory is complete.
- Add import guidance for user dictionary installation after native file picker and app data folder UX are designed.
- Expand UI translations after a contribution workflow and review policy are accepted.
