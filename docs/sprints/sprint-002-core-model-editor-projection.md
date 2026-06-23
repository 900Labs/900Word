# Sprint 002: Core Model And Editor Projection

## Scope

Strengthen the canonical document model and editor projection boundary so frontend editing cannot create structures that `word-core` does not understand.

## Deliverables

- `word-core` exposes document commands, undo/redo, stats, and style-registry lookup/registration tests.
- Generated JSON fixture coverage exists in `crates/word-fixtures`.
- The desktop editor uses a constrained ProseMirror schema instead of the broad basic schema.
- Frontend projection tests verify supported `word-core` blocks and inline marks map to and from ProseMirror JSON.
- Editor changes are converted into `DocumentCommand` values and submitted through the existing Tauri IPC boundary.
- Modeled blocks outside the Sprint 002 editor projection open read-only with warnings instead of being silently dropped.
- Link projections reject unsafe schemes before DOM parsing and JSON schema loading.
- Unsupported ProseMirror nodes are rejected by schema tests.

## Validation

Run from the repository root:

```bash
npm run check
npm run lint
npm run test
cargo fmt --all -- --check
cargo test --workspace
./scripts/verify-public-release.sh
```

## Evidence

- `crates/word-core/src/lib.rs` contains model, command, undo/redo, stats, and style-registry tests.
- `crates/word-fixtures/fixtures/generated-multilingual.json` is a synthetic JSON fixture.
- `apps/desktop/src/lib/editorSchema.ts` defines the constrained ProseMirror schema.
- `apps/desktop/src/lib/documentProjection.test.ts` verifies `word-core` to editor JSON projection.
- `apps/desktop/src/lib/editorSchema.test.ts` verifies unsupported ProseMirror nodes are rejected.
- `apps/desktop/src/App.svelte` queues editor changes through `apply_document_command` for the editable projection.

## Follow-Ups

- Sprint 003 owns ODT golden round-trip expansion for lists, tables, images, styles, RTL text, and CJK text.
- Sprint 004 owns durable file workflow state such as autosave, recovery, and recent files.
