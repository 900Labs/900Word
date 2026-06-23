# Sprint 005: Editing Completeness

## Scope

Add first-pass editing workflow controls without widening the supported ProseMirror projection or enabling unsupported document structures.

## Deliverables

- Desktop toolbar controls for undo, redo, bold, italic, underline, strikethrough, superscript, subscript, paragraph style, and heading levels 1-2.
- Keyboard shortcuts for save, undo, redo, find focus, bold, italic, and underline.
- Find/replace controls with previous/next navigation, case-sensitive matching, replace current, and replace all.
- Generated starter templates for blank, report, and letter documents. Templates contain placeholder-only text and no real user documents.
- Page setup controls for width, height, and margins, backed by validated `word-core` section metadata and basic ODT page-layout round-trip coverage.
- Workspace tab traversal with arrow, Home, and End keys.
- Tests for page setup validation, ODT page setup round-trip, generated template behavior, and find/replace matching.

## Validation

Run from the repository root:

```bash
npm run check
npm run lint
npm run test
cargo test -p word-core
cargo test -p nine-hundred-word
./scripts/verify-public-release.sh
```

## Evidence

- `crates/word-core/src/lib.rs` adds validated `UpdatePageSetup` command handling.
- `crates/word-odf/src/lib.rs` serializes and imports basic page setup metadata.
- `apps/desktop/src-tauri/src/lib.rs` adds template listing and new-from-template commands.
- `apps/desktop/src/App.svelte` exposes toolbar, template, find/replace, page setup, shortcut, and tab traversal workflows.
- `apps/desktop/src/lib/findReplace.ts` contains tested find-range logic.

## Follow-Ups

- Full ODT page-layout fidelity and deterministic pagination remain future file-format compatibility tasks.
- Toolbar state reflection for active marks and block type remains future polish.
- Broader block projection for lists, tables, images, and page breaks remains gated on durable `word-core` semantics and round-trip tests.
