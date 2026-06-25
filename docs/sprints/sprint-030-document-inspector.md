# Sprint 030: Document Inspector MVP

## Status

Complete.

## Goal

Expose a lightweight local-first Document Inspector that summarizes document format, save state, core counts, embedded image payload size, review indicators, notes, and privacy warnings without exposing private local paths or adding new persistence, telemetry, accounts, cloud, network behavior, or heavy dependencies.

## Shipped

- Added a tested desktop `documentInspector` projection helper for aggregate inspector summaries.
- Added a File menu command and bottom-toolbar button for the Document Inspector panel.
- Kept saved-location details generic: the frontend shows only whether a saved location exists backend-side, never the local path or filename.
- Showed canonical format as OpenDocument Text (`.odt`).
- Showed saved/unsaved status, created/modified metadata timestamps, page size, words, characters, paragraphs, model blocks, estimated pages, and selection words.
- Showed embedded image count and total embedded image bytes from in-document image assets.
- Showed comments, unresolved comments, track changes recording status, tracked change count, footnotes, and endnotes.
- Added privacy warnings for comments, tracked changes, document title metadata, local recovery drafts, and unsaved state.
- Kept the sprint frontend-only; no `word-core`, ODT, export, import, account, telemetry, cloud, network, or heavy dependency changes were added.

## Compatibility Boundary

- The inspector is an ephemeral desktop UI projection over existing local document state.
- The format indicator is intentionally `.odt` canonical and does not add DOCX/PDF inspection claims.
- Embedded image bytes are derived from local document assets and are aggregate values only. Source paths and source filenames are not displayed.
- The saved-location field is a generic policy/status indicator, not a path display.
- Estimated pages remain lightweight estimates and do not imply deterministic pagination or layout fidelity.
- Privacy warnings are local UI guidance only; they do not encrypt recovery drafts, remove metadata, or automatically accept/reject review content.

## Deferred

- Full package-level metadata inspection, ODT manifest inspection, document cleanup tools, image compression, path reveal controls, deterministic pagination, and exportable inspector reports.
- Component-level browser tests for the Svelte popover; the MVP covers the projection helper plus Svelte/TypeScript checks.

## Verification

- `npm --workspace apps/desktop run test -- documentInspector` - required for helper summary, byte formatting, warning kinds, and no private path/source-name leakage.
- `npm --workspace apps/desktop run check` - required for Svelte/TypeScript integration.
- `npm --workspace apps/desktop run lint` - required for TypeScript integration.
- `git diff --check` - required for whitespace safety.

## Privacy Notes

- The inspector displays document-created and document-modified metadata timestamps when present, but it does not display local paths, private filenames, source image filenames, usernames, hostnames, contacts, cloud identities, recovery locations, or document text outside already visible editor content.
- Saved file paths remain backend-only.
- Recovery drafts remain local and unencrypted; the inspector warns when recovery summaries exist but still shows only generic recovery state.
- No telemetry, accounts, cloud sync, remote lookup, AI service, network behavior, document-content logging, export metadata, or saved document metadata was added.
