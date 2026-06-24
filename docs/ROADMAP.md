# Roadmap

## Sprint 000: Governance And Privacy Bootstrap

Status: complete.

- Initialize Git and remote.
- Add public documentation, ADRs, CI skeleton, release skeleton, branch policy, PR template, and privacy scanner.
- Add this sprint record.

## Sprint 001: Runnable Shell And Workspace

Status: complete.

- Scaffold Rust workspace and Tauri/Svelte app.
- Add strict CSP, least-privilege capabilities, and no shell plugin in core.
- Add placeholder editor, settings, About/license view, and no-network startup smoke.

## Sprint 002: Core Model And Editor Projection

Status: complete.

- Implement `word-core` model, commands, undo/redo, word count, styles, and JSON fixtures.
- Add ProseMirror schema that maps only to supported `word-core` structures.

## Sprint 003: ODT MVP Round-Trip

Status: complete.

- Implement ODT package open/save for paragraphs, runs, headings, lists, tables, images, metadata, and styles.
- Add round-trip fixtures for RTL and CJK text.

## Sprint 004: Local File Workflows

Status: complete.

- Add open/save/save-as, manual autosave, recovery drafts, recent files, and hostile-file validation.

## Sprint 005: Editing Completeness

Status: complete.

- Add toolbar, keyboard shortcuts, find/replace, page setup metadata, templates, and accessibility traversal.

## Sprint 006: Language Tools

Status: complete.

- Add Hunspell-shaped word-list boundary, bundled minimal dictionaries, user dictionaries, dictionary license inventory, and i18n.

## Sprint 007: Export And Print

Status: complete.

- Add TXT, sanitized HTML, and basic PDF export-to-path workflows.
- Add sanitized print HTML and WebView print action.
- Keep deterministic pagination deferred.

## Sprint 008: Release Hardening

Status: complete.

- Add performance smoke, bundle-size budget, package privacy scan, runtime offline scan, SBOM, and release checklist.

## Sprint 009: Authoring Foundation

Status: complete.

- Expand the durable style registry, paragraph formatting, inline text style, list definitions, and editor projection.
- Add toolbar controls for styles, font controls, paragraph controls, lists, and clear formatting.
- Add generated ODT and sanitized HTML handling for 900Word-authored direct formatting.
- Keep full arbitrary style editing and a persistent style panel deferred.

## Sprint 010: Authoring Polish

Status: complete.

- Add active formatting detection from the current selection.
- Add update-style-from-selection for selected paragraph style properties.
- Add list-aware Enter behavior and basic plain-text list paste handling.
- Add Hunspell-backed misspelling underlines, suggestions, ignore actions, and local personal dictionary additions.
- Add selection word count and an expanded local stats panel.
- Keep rich clipboard import, full style editing, and a dictionary manager deferred.

## Sprint 011: Structure, Navigation, And Links

Status: complete.

- Add a live Heading 1/2/3 navigator sidebar with click-to-jump behavior.
- Add insert/edit/remove hyperlink UI backed by the existing safe link model and ODT/export path.
- Keep bookmarks, headers/footers/fields, and image insertion workflows deferred.

## Sprint 012: Editable Tables MVP

Status: complete.

- Project `word-core` table blocks into editable desktop table nodes when cells contain paragraphs, headings, or lists.
- Sync edited table cell text and supported cell blocks back through the existing document command path.
- Add a default 2x2 insert-table toolbar control.
- Keep row/column structural editing, delete-table controls, merged cells, table styling, and unsupported nested cell content deferred.
