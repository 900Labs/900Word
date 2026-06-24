# Sprint 009: Authoring Foundation

Status: complete.

## Scope

This sprint starts the significant word-processor upgrade layer from the upgrade plan. It focuses on the first durable authoring slice: richer style metadata, direct text and paragraph formatting, editable list projection, and broader keyboard/toolbar access.

## Completed In This Slice

- Expanded `word-core` style registry with paragraph styles `Normal`, `Title`, `Subtitle`, `Heading 1`, `Heading 2`, `Heading 3`, `Quote`, `Code`, and `Caption`.
- Added character style registry entries for `Emphasis`, `Strong`, `Link`, and `Highlight`, plus page style entries for default, first page, landscape, and letterhead.
- Added durable `ParagraphFormat` and `InlineStyle` fields for alignment, line spacing, spacing, indents, font family, font size, text color, and highlight color.
- Added default ordered and unordered list definitions.
- Extended the ProseMirror projection with real unordered/ordered list nodes and list item levels.
- Added toolbar controls for style selection, clear formatting, H3, font family, font size, text color, highlight color, alignment, line spacing, paragraph spacing, first-line indent, bullets, numbering, and list level changes.
- Added keyboard shortcuts for heading levels 1-3, bullet/numbered lists, and list level changes.
- Enabled low-cost native WebView spellcheck attributes while preserving the offline Hunspell status workflow.
- Extended generated ODT output so 900Word-authored paragraph direct formatting, inline font/color/highlight, and list levels survive save/reopen.
- Extended sanitized HTML/print export to include paragraph and inline direct formatting.

## Verification

- `npm run lint`
- `npm run check`
- `npm run test`
- `cargo test -p word-odf authoring_formatting_round_trips_through_generated_odt_styles`
- `cargo test -p word-export`
- `cargo test --workspace`

## Follow-On Work Moved To Sprint 010

- Sprint 010 implements update-style-from-selection for selected paragraph style properties, active formatting detection, list continuation/exit behavior, basic list paste handling, Hunspell-backed red underline decorations, spelling suggestions, ignore actions, personal dictionary writes, selection word count, and an expanded stats panel.
- Full arbitrary style editing, a persistent style panel, rich clipboard import, and visual smoke testing of the rebuilt toolbar remain follow-on work.

## Privacy Notes

- No telemetry, accounts, cloud calls, local paths, private filenames, or real documents were added.
- New fixtures and tests use synthetic text only.
- Generated ODT style names encode only bounded formatting tokens, not user content.
