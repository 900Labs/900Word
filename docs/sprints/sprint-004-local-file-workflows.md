# Sprint 004: Local File Workflows

## Scope

Implement local document file workflows without adding broad filesystem, shell, or network permissions.

## Deliverables

- Backend commands support new, open, save, save-as, open-recent, manual autosave, recovery listing, recovery open, recovery discard, and file-state queries.
- User-selected `.odt` paths are validated for extension, traversal components, and size limits before read/write operations.
- Saves use size-checked atomic writes.
- Recent documents keep real paths backend-only and expose opaque tokens plus generic labels to the frontend.
- Autosave originally wrote one sanitized recovery draft per document. Sprint 029 replaces new autosaves with bounded versioned recovery snapshots while preserving validated legacy recovery tokens for list, recover, and discard.
- On Unix platforms, recovery directories and files are forced to owner-only permissions.
- Recovery drafts open as dirty unsaved documents rather than adopting the recovery file as the save path.
- The desktop shell uses native dialogs for ODT Open and Save As with scoped dialog permissions.
- Tests cover path traversal rejection, wrong extension rejection, recovery token validation, recent-summary privacy, output-size rejection, and private recovery-style file permissions on Unix.

## Validation

Run from the repository root:

```bash
npm run check
npm run lint
npm run test
cargo test -p nine-hundred-word
cargo test --workspace
./scripts/verify-public-release.sh
```

## Evidence

- `apps/desktop/src-tauri/src/lib.rs` contains local file workflow commands, recovery token validation, recent summaries, and tests.
- `apps/desktop/src/App.svelte` exposes file controls and recovery/recent actions without displaying local paths.
- `apps/desktop/src-tauri/capabilities/default.json` allows only scoped open/save dialog permissions in addition to the core shell permissions.
- `docs/PRIVACY_MODEL.md` documents recent and recovery privacy behavior.

## Follow-Ups

- Export path dialogs remain future work; Sprint 007 still uses explicit export paths.
- Recovery encryption remains deferred until the encryption ADR is implemented and tested.
- Periodic autosave, crash hooks, close handling, and dirty prompts remain Sprint 005+ editing/workflow work.
