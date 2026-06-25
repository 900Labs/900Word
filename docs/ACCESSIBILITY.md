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

Accessibility smoke checks are part of the release hardening roadmap.
