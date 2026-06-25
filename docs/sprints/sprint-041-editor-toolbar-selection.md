# Sprint 041: Editor Toolbar Selection Fixes

## Status

Complete.

## Goal

Fix toolbar formatting commands so they apply to the user's selected editor text or current block even when toolbar interaction would otherwise move focus away from the editor.

## Shipped

- Added a mouse fallback for the core toolbar formatting buttons so desktop mouse activation preserves the editor selection before the browser can move focus to the toolbar.
- Kept pointer activation as the primary path and deduplicated pointer/mouse handling so a single click applies a command once.
- Covered selected-text toolbar command behavior for bold, italic, underline, superscript, and subscript when the live editor selection has collapsed but the saved toolbar selection is still available.
- Covered selected-block toolbar command behavior for Paragraph, H1, H2, and H3 through the same saved-selection command path.

## Verification

- `npm run check`
- `npm run test --workspace apps/desktop`
- `git diff --check`

## Privacy Notes

- This sprint changes local editor interaction only.
- No document content logging, telemetry, network access, accounts, cloud sync, file-path exposure, import/export changes, or document metadata changes were added.
