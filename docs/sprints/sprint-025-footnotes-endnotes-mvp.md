# Sprint 025: Footnotes And Endnotes MVP

## Status

Complete.

## Goal

Add bounded local footnotes and endnotes for academic and legal-style drafting while keeping `word-core` canonical, ODT-native, local-first, privacy-safe, and explicit that deterministic pagination and page-bottom note layout remain deferred.

## Shipped

- Added durable `word-core` `Note` metadata plus inline note references for footnotes and endnotes with bounded IDs, labels, bodies, and note count.
- Added note commands for inserting, adding, updating, and deleting notes, including undo/redo coverage through the existing `UndoStack`.
- Included note references and note body text in document text/stat handling so notes are not silently excluded from core text accounting.
- Added ProseMirror `note_reference` inline atoms so desktop editing preserves note references instead of flattening them to ordinary text.
- Added compact Footnote and Endnote toolbar controls with simple local body entry plus a Notes sidebar that surfaces stored note bodies beside inline references.
- Preserved 900Word-authored notes in ODT using `text:note`, `text:note-citation`, and `text:note-body` elements with bounded `word900` note ID/kind metadata.
- Promoted bounded ODT notes with matching safe `word900` metadata into local notes and surfaced their bodies in the Notes sidebar; unsupported, malformed, duplicate, over-limit, or unanchored notes import as visible fallback text with generic warnings.
- Added conservative TXT, sanitized HTML, print HTML, and basic PDF export output for inline note references and appended Footnotes/Endnotes body sections.

## Compatibility Boundary

- Footnotes and endnotes are local 900Word note references and plain text bodies, not a full note-management system.
- 900Word does not claim deterministic pagination, page-bottom footnote placement, note continuation, or PDF note layout fidelity.
- ODT note promotion requires bounded, safe `word900` note ID/kind metadata that matches the ODF note kind and a simple note body. Promoted note bodies are visible in the Notes sidebar. Unsupported notes remain visible as ordinary text with warnings.
- HTML and print HTML include sanitized note sections. Basic PDF remains text-oriented.

## Deferred

- Deterministic pagination and page-bottom layout.
- Rich note editing panels, renumbering controls, cross-references, note continuation, and custom note styles.
- Full external ODT note interoperability, DOCX notes, active PDF note annotations, and legal citation automation.

## Verification

- `cargo test -p word-core footnote` - passed.
- `cargo test -p word-odf note` - passed.
- `cargo test -p word-export note` - passed.
- `npm --workspace apps/desktop run test -- src/lib/documentProjection.test.ts src/lib/editorSchema.test.ts src/lib/editor.test.ts` - passed.
- `./scripts/verify-local.sh` - passed after builder/reviewer/control-tower fixes.

## Privacy Notes

- Notes are local document content only.
- Generated note IDs are compact document-local identifiers and do not include usernames, hostnames, account identifiers, contacts, local paths, source filenames, cloud identity, network state, or external services.
- The feature adds no telemetry, accounts, cloud sync, remote lookup, remote assets, or document-content logging.
