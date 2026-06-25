# Sprint 056: DOCX Table Of Contents Interoperability

## Goal

Add bounded DOCX interoperability for 900Word-authored generated table-of-contents blocks while keeping ODT canonical, `word-core` as the source of truth, and Word-native TOC field-code fidelity deferred.

## Completed

- Export 900Word-generated TOCs as visible DOCX paragraphs using `Word900TocTitle` and `Word900TocEntry1`-`Word900TocEntry3` styles.
- Export active TOC row links only when the target bookmark is safe, unique, and emitted as a DOCX bookmark marker.
- Export unique safe paragraph and heading bookmark IDs as minimal `bookmarkStart` / `bookmarkEnd` pairs with generated numeric IDs.
- Import safe paragraph and heading `bookmarkStart` names into `word-core` bookmark IDs.
- Import the generated 900Word TOC style plus safe internal-link shape back into `word-core` `TableOfContents` blocks.
- Keep styled TOC rows without safe internal links as visible paragraphs rather than hidden structured content.

## Boundaries

- ODT remains the canonical saved format.
- DOCX remains conversion-only; opening `.docx` creates an unsaved dirty document.
- Word field-code TOCs, automatic refresh metadata, deterministic TOC page numbers, complex cross references, arbitrary Word TOC styles, and full DOCX layout fidelity remain deferred.
- Duplicate or unsafe bookmark IDs do not become DOCX bookmark targets.

## Verification

- `cargo check -p word-docx`
- `cargo test -p word-docx`

## Privacy Notes

- DOCX bookmark export writes only compact safe document-local bookmark names and generated numeric IDs.
- Generated TOC styles and links do not include local paths, private filenames, usernames, hostnames, account identifiers, cloud identity, telemetry identifiers, network state, or source document names.
- The feature adds no telemetry, network calls, cloud sync, accounts, remote resource fetching, plugin behavior, or heavy dependencies.
