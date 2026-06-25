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

Accessibility smoke checks are part of the release hardening roadmap.
