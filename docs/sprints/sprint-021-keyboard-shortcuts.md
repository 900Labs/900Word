# Sprint 021: Keyboard Shortcut Polish

## Status

Complete.

## Goal

Polish standard keyboard shortcuts for existing 900Word desktop features without adding dependencies, telemetry, file-format behavior, document model changes, or new feature claims.

## Shipped

- Added a small tested shortcut helper for platform-neutral label normalization and command identification from KeyboardEvent-like plain objects.
- Kept existing shortcuts for new, open, save, save as, print, undo, redo, bold, italic, underline, headings, link editing, lists, indent, and find.
- Added Cmd/Ctrl+Y as an additional redo shortcut alongside Cmd/Ctrl+Shift+Z.
- Added a replace shortcut that opens the existing find/replace popover and focuses the existing replace field.
- Added an Export PDF shortcut. When a `.pdf` export path is already present, it runs the existing PDF export command; otherwise it opens the existing File > Export path flow and asks for a PDF export path with generic status text.
- Added shortcut hints to the File menu, PDF export button, history controls, formatting controls, heading controls, link control, list controls, indent controls, and find/replace search toggle.
- Added input-target guard coverage so editor-destructive shortcuts do not fire from inputs, textareas, or selects, while save, save as, print, and find remain available.

## Deferred

- Insert-comment shortcuts or comments UI.
- User-configurable shortcut maps.
- Native menu accelerators.
- File-format metadata, persistent preferences, telemetry, or document model changes.
- Broader find/replace panel redesign.

## Verification

- `npm run check`
- `npm run lint`
- `npm run test`
- `git diff --check`
- `./scripts/verify-public-release.sh`

## Privacy Notes

- Shortcut handling is local UI behavior only.
- Export PDF continues to use the existing explicit export path input and existing export command.
- No local paths, filenames, user identities, remote assets, network calls, telemetry, or broad file writes were added.
