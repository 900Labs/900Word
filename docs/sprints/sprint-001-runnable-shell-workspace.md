# Sprint 001: Runnable Shell And Workspace

## Scope

Turn the bootstrap into a runnable local desktop shell with explicit workspace boundaries, minimal Tauri permissions, and the first user-visible editor, settings, and About surfaces.

## Deliverables

- Rust workspace remains split across `apps/desktop` and `crates/*`.
- Tauri v2 desktop shell uses a non-null CSP and the default capability grants only `core:default`.
- Core application dependencies do not include the Tauri shell plugin.
- Svelte 5 UI exposes Editor, Settings, and About views.
- Settings shell keeps telemetry disabled and supports language and high-contrast state.
- About view exposes version, license, native document format, and telemetry state.
- Startup smoke tests verify the frontend boot path does not use browser network primitives.

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

- `apps/desktop/src-tauri/tauri.conf.json` contains the desktop CSP.
- `apps/desktop/src-tauri/capabilities/default.json` grants only `core:default`.
- `apps/desktop/src/App.svelte` contains the Editor, Settings, and About views.
- `apps/desktop/src-tauri/src/lib.rs` contains tests for telemetry, startup network primitives, and default shell access.

## Follow-Ups

- Sprint 002 owns the richer `word-core` command projection and ProseMirror schema boundaries.
- Sprint 004 owns persistent user settings, recent files, autosave, and recovery.
