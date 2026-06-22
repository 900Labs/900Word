# Roadmap

## Sprint 000: Governance And Privacy Bootstrap

- Initialize Git and remote.
- Add public documentation, ADRs, CI skeleton, release skeleton, branch policy, PR template, and privacy scanner.
- Add this sprint record.

## Sprint 001: Runnable Shell And Workspace

- Scaffold Rust workspace and Tauri/Svelte app.
- Add strict CSP, least-privilege capabilities, and no shell plugin in core.
- Add placeholder editor, settings, About/license view, and no-network startup smoke.

## Sprint 002: Core Model And Editor Projection

- Implement `word-core` model, commands, undo/redo, word count, styles, and JSON fixtures.
- Add ProseMirror schema that maps only to supported `word-core` structures.

## Sprint 003: ODT MVP Round-Trip

- Implement ODT package open/save for paragraphs, runs, headings, lists, tables, images, metadata, and styles.
- Add round-trip fixtures for RTL and CJK text.

## Sprint 004: Local File Workflows

- Add open/save/save-as, autosave, crash recovery, recent files, and hostile-file validation.

## Sprint 005: Editing Completeness

- Add toolbar, keyboard shortcuts, find/replace, page setup metadata, templates, and accessibility traversal.

## Sprint 006: Language Tools

- Add Hunspell boundary, bundled minimal dictionaries, user dictionaries, dictionary license inventory, and i18n.

## Sprint 007: Export And Print

- Add TXT, sanitized HTML, and basic PDF export.
- Keep deterministic pagination deferred.

## Sprint 008: Release Hardening

- Add performance smoke, bundle-size budget, package privacy scan, runtime offline scan, SBOM, and release checklist.
