# Accessibility

Accessibility is a release requirement, not a polish task.

Initial requirements:

- Keyboard access to editor, toolbar, settings, and About views.
- Visible focus states.
- High-contrast theme.
- Screen-reader labels for controls.
- No hover-only commands.
- Text that fits at common desktop and laptop widths.

Sprint 005 adds keyboard traversal for workspace tabs with arrow, Home, and End keys. The editing toolbar and find/replace controls are button/input based, have labels or stable button text, and do not rely on hover-only actions.

Sprint 021 adds a tested shortcut helper for standard desktop command shortcuts, consistent visible hints in menus and tooltips, and guarded behavior for form fields. Editor-destructive shortcuts such as new, open, formatting, list, indent, link, undo, redo, replace, and export are not fired while focus is inside inputs, textareas, or selects; standard low-risk app commands such as save, save as, print, and find remain available globally.

Sprint 022 adds a comments sidebar using native buttons and textareas. Comment commands are not hover-only: users can open the panel with the toolbar or Insert Comment shortcut, enter a bounded comment body, add it to non-empty selected text, jump to a comment anchor, resolve/reopen, and delete with keyboard-focusable controls. Commented text receives a visible inline marker in the editor.

Sprint 023 adds a track changes toolbar toggle and review sidebar using native checkbox and button controls. Accept/reject, accept all/reject all, and jump-to-change actions are keyboard-focusable and not hover-only. Insertions and deletions receive visible inline text styling in addition to the review list labels.

Sprint 024 adds the table-of-contents insert/update command inside the existing File menu as a native button. Generated TOC blocks render in the editor as a visible document block with normal focusable internal links where safe bookmark targets exist; the command and links are not hover-only.

Sprint 025 adds footnote and endnote insert buttons to the existing review/comments toolbar group plus a compact Notes sidebar for stored note bodies. The controls are native buttons with stable labels and are not hover-only. Inserted references render as visible inline superscript atoms with screen-reader labels for the note kind and visible label; note body entry uses a simple local prompt in this MVP, and existing note bodies are readable through keyboard-focusable sidebar content.

Sprint 026 adds Smart typing settings as native checkboxes in the Settings view. The toggles have stable labels, are keyboard-focusable, default off, and do not rely on hover-only behavior. Typed-input transforms use the existing editor surface and do not add modal interruptions or hidden background services.

Accessibility smoke checks are part of the release hardening roadmap.
