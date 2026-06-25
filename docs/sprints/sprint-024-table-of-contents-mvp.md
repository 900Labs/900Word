# Sprint 024: Table Of Contents MVP

## Status

Complete.

## Goal

Add a bounded local-first table-of-contents feature based on supported 900Word headings and bookmarks while keeping `word-core` canonical and avoiding deterministic page-number, live-pagination, cloud, telemetry, identity, or full external ODT TOC compatibility claims.

## Shipped

- Added a durable `TableOfContents` block to `word-core` with generated entries containing heading text, heading level, and a safe internal bookmark target.
- Added an `insert_or_update_table_of_contents` document command that derives entries from supported top-level Heading 1-3 blocks in the editable first-section projection.
- Reuses existing unique safe heading bookmark IDs and generates document-local `toc-*` bookmark IDs when headings have missing, unsafe, or duplicate targets.
- Added a desktop File-menu command to insert or update contents from headings without adding a new top-level toolbar surface.
- Added a ProseMirror `table_of_contents` atom that renders as a visible local contents block with keyboard-focusable internal links where safe targets exist.
- Preserved 900Word-authored TOCs through ODT save/reopen with `word900:block-type="table-of-contents"` metadata on visible generated text.
- Promotes imported TOC metadata only when bounded metadata exactly matches the visible generated text and safe local fragment links; mismatched or oversized metadata imports as visible paragraph text with a generic warning.
- Added TXT/basic PDF text export and HTML/print HTML local fragment-link export behavior.

## Compatibility Boundary

- TOCs are generated 900Word document blocks, not a claim of full ODT-native TOC interoperability.
- Entries are derived from supported top-level Heading 1-3 blocks in the first editable section only.
- TOC entries store heading text and safe bookmark targets; they do not store or compute deterministic page numbers.
- If another ODT editor strips `word900` metadata, the visible generated text remains readable, but 900Word may no longer recognize it as a TOC block.
- If external ODT content adds hidden or mismatched TOC metadata, 900Word does not promote it into generated TOC content.
- HTML and print HTML export safe internal links where bookmark targets are safe. TXT and basic PDF export ordinary visible text.

## Deferred

- Deterministic page numbers, live pagination, and page-number refresh.
- Full external ODT TOC import/export compatibility and automatic refresh by other editors.
- TOCs across multiple sections, headings nested inside tables/lists, custom TOC styles, and multiple independent TOCs.
- DOCX TOCs, active PDF link annotations, and rich TOC editing UI.

## Verification

- `npm run check` - passed.
- `npm run lint` - passed.
- `npm run test` - passed.
- `cargo fmt --all -- --check` - passed.
- `cargo test --workspace` - passed.
- `git diff --check` - passed.
- `./scripts/verify-local.sh` - passed.
- `./scripts/verify-public-release.sh` - passed.

## Privacy Notes

- TOC generation reads only the local in-memory document model.
- Generated bookmark IDs are compact document-local identifiers and do not include usernames, hostnames, account identifiers, contacts, local paths, source filenames, cloud identity, network state, or external services.
- The feature adds no telemetry, accounts, cloud sync, remote lookup, or network behavior.
