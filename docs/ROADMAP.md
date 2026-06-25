# Roadmap

## Sprint 000: Governance And Privacy Bootstrap

Status: complete.

- Initialize Git and remote.
- Add public documentation, ADRs, CI skeleton, release skeleton, branch policy, PR template, and privacy scanner.
- Add this sprint record.

## Sprint 001: Runnable Shell And Workspace

Status: complete.

- Scaffold Rust workspace and Tauri/Svelte app.
- Add strict CSP, least-privilege capabilities, and no shell plugin in core.
- Add placeholder editor, settings, About/license view, and no-network startup smoke.

## Sprint 002: Core Model And Editor Projection

Status: complete.

- Implement `word-core` model, commands, undo/redo, word count, styles, and JSON fixtures.
- Add ProseMirror schema that maps only to supported `word-core` structures.

## Sprint 003: ODT MVP Round-Trip

Status: complete.

- Implement ODT package open/save for paragraphs, runs, headings, lists, tables, images, metadata, and styles.
- Add round-trip fixtures for RTL and CJK text.

## Sprint 004: Local File Workflows

Status: complete.

- Add open/save/save-as, manual autosave, recovery drafts, recent files, and hostile-file validation.

## Sprint 005: Editing Completeness

Status: complete.

- Add toolbar, keyboard shortcuts, find/replace, page setup metadata, templates, and accessibility traversal.

## Sprint 006: Language Tools

Status: complete.

- Add Hunspell-shaped word-list boundary, bundled minimal dictionaries, user dictionaries, dictionary license inventory, and i18n.

## Sprint 007: Export And Print

Status: complete.

- Add TXT, sanitized HTML, and basic PDF export-to-path workflows.
- Add sanitized print HTML and WebView print action.
- Keep deterministic pagination deferred.

## Sprint 008: Release Hardening

Status: complete.

- Add performance smoke, bundle-size budget, package privacy scan, runtime offline scan, SBOM, and release checklist.

## Sprint 009: Authoring Foundation

Status: complete.

- Expand the durable style registry, paragraph formatting, inline text style, list definitions, and editor projection.
- Add toolbar controls for styles, font controls, paragraph controls, lists, and clear formatting.
- Add generated ODT and sanitized HTML handling for 900Word-authored direct formatting.
- Keep full arbitrary style editing and a persistent style panel deferred.

## Sprint 010: Authoring Polish

Status: complete.

- Add active formatting detection from the current selection.
- Add update-style-from-selection for selected paragraph style properties.
- Add list-aware Enter behavior and basic plain-text list paste handling.
- Add Hunspell-backed misspelling underlines, suggestions, ignore actions, and local personal dictionary additions.
- Add selection word count and an expanded local stats panel.
- Keep rich clipboard import, full style editing, and a dictionary manager deferred.

## Sprint 011: Structure, Navigation, And Links

Status: complete.

- Add a live Heading 1/2/3 navigator sidebar with click-to-jump behavior.
- Add insert/edit/remove hyperlink UI backed by the existing safe link model and ODT/export path.
- Keep headers/footers/fields and image insertion workflows deferred.

## Sprint 012: Editable Tables MVP

Status: complete.

- Project `word-core` table blocks into editable desktop table nodes when cells contain paragraphs, headings, or lists.
- Sync edited table cell text and supported cell blocks back through the existing document command path.
- Add a default 2x2 insert-table toolbar control.
- Keep row/column structural editing, delete-table controls, merged cells, table styling, and unsupported nested cell content deferred.

## Sprint 013: Headers, Footers, And Page Fields

Status: complete.

- Add section-level header/footer page regions to `word-core`.
- Add simple Settings-panel editing for headers, footers, first-page variants, and page-field insertion.
- Persist 900Word-authored page regions and fields through ODT and include them in TXT, HTML, print HTML, and basic PDF exports.
- Keep deterministic pagination, rich region editing, and complex external header/footer fidelity deferred.

## Sprint 014: Local Image Insertion

Status: complete.

- Add a native desktop Insert Image command for local PNG, JPEG, GIF, and WebP files.
- Validate local image paths, extensions, byte size, and magic bytes in Rust before import.
- Store accepted bytes as embedded `word-core` assets and insert `ImageBlock` references without preserving source paths or private source filenames.
- Project images as non-editable ProseMirror image atoms so text editing does not silently drop image blocks.
- Persist inserted images through ODT save/reopen and include allowlisted embedded image assets in offline HTML export as data URLs.
- Keep dedicated alt text editing, image resizing/cropping, and raster PDF embedding deferred.

## Sprint 015: Image Polish MVP

Status: complete.

- Add durable image presentation metadata to `word-core`: editable alt text, caption text, alignment, and bounded scale percentage.
- Add compact contextual desktop controls for the selected image atom without introducing a custom image editor framework.
- Preserve image metadata through ProseMirror projection/text sync and 900Word-authored ODT save/reopen using the existing `word900` metadata namespace.
- Reflect image alt/caption/alignment/scale in sanitized HTML and print HTML export.
- Keep cropping, drag resize handles, richer external ODT image-layout fidelity, and raster PDF image embedding deferred.

## Sprint 016: Bookmarks And Internal Links MVP

Status: complete.

- Add optional safe bookmark IDs to durable paragraph and heading blocks.
- Preserve bookmark IDs through ProseMirror projection and editor sync.
- Add compact controls to create/remove a bookmark on the selected paragraph or heading and link selected text to an existing bookmark/heading target.
- Preserve 900Word-authored bookmarks and internal links through ODT using `text:bookmark` anchors and `#fragment` text links.
- Emit sanitized HTML element IDs and internal fragment hrefs.
- Keep richer bookmark management, automatic heading ID assignment, cross-document links, and active PDF link annotations deferred.

## Sprint 017: Table Structure Editing MVP

Status: complete.

- Replace the fixed 2x2 table command with a compact bounded row/column insert control.
- Add contextual desktop controls for add row above/below, delete row, add column left/right, delete column, and delete table when the selection is inside an editable rectangular table.
- Keep table edits inside the existing ProseMirror projection and `word-core` block sync path without changing the durable Rust table model.
- Keep merged cells, resizing, formulas, rich table paste, and heavy table styling deferred.

## Sprint 018: Image Resize UX

Status: complete.

- Add a direct drag handle on selected image atoms that updates durable bounded `scalePercent` image presentation metadata.
- Keep existing toolbar image scale controls in sync with direct resize by using the same selected-image transaction path.
- Surface generic oversized-image import guidance without exposing local paths, source filenames, usernames, or filesystem details.
- Keep source images embedded in document assets; no external links, remote loads, or raster PDF embedding claims are added.
- Keep crop/rotation/compression, native pixel sizing, and richer external ODT image-layout fidelity deferred.

## Sprint 019: Template Gallery

Status: complete.

- Expand generated starter templates beyond blank/report/letter while preserving those existing stable IDs.
- Add project report, CV/resume, meeting minutes, memo, invoice-style, and flyer one-pagers using supported `word-core` blocks.
- Keep templates generated in code with placeholder-only text, no local paths, no real documents, no remote assets, and no external template files.
- Keep richer template browsing, template previews, custom user templates, and downloadable template packs deferred.

## Sprint 020: Page View And Zoom

Status: complete.

- Add local editor viewport controls for Draft and Page Layout modes.
- Add lightweight zoom controls for Fit Width, 100%, and bounded custom zoom.
- Add simple visual ruler guides derived from the current page setup and margins.
- Keep the controls as editor viewport behavior only; deterministic pagination, page-break preview, and print layout fidelity remain deferred.

## Sprint 021: Keyboard Shortcut Polish

Status: complete.

- Add a tested desktop shortcut helper for platform-neutral labels, command matching, and input-target guards.
- Preserve existing authoring and file shortcuts while adding replace, redo via Cmd/Ctrl+Y, and Export PDF handling through the existing export path flow.
- Surface shortcut hints in File menu commands, export controls, and toolbar tooltips.
- Keep comments, custom shortcut preferences, telemetry, dependencies, file-format changes, and model changes deferred.

## Sprint 022: Comments MVP

Status: complete.

- Add bounded durable local comment threads to `word-core` with inline selected-text anchors.
- Add desktop comments sidebar controls for add, jump, resolve/reopen, and delete.
- Preserve comment marks through ProseMirror projection alongside formatting, links, and direct text style metadata.
- Save and reopen 900Word-authored comments in ODT using ODF annotation elements with `word900` metadata for local ID and resolved state.
- Add an Insert Comment shortcut that opens the comments panel and focuses the comment body.
- Keep thread replies, multiple authors, comment search, full external ODT annotation compatibility, DOCX comments, and PDF annotation export deferred.

## Sprint 023: Track Changes MVP

Status: complete.

- Add document-level local Record changes state with a compact desktop review panel.
- Track inserted text as visible local insertions and selected Backspace/Delete text as visible local deletions instead of removing selected text immediately.
- Add individual accept/reject plus accept all/reject all commands backed by `word-core` cleanup semantics.
- Store privacy-safe local author metadata as `Local User` plus timestamps without reading OS usernames, hostnames, accounts, paths, contacts, or cloud identity.
- Preserve 900Word-authored text-only tracked changes through ODT save/reopen with `word900` metadata.
- Keep formatting-only changes, table structure changes, image changes, comments-as-changes, DOCX track changes, compare/merge, and multi-author collaboration deferred.

## Sprint 024: Table Of Contents MVP

Status: complete.

- Add a durable generated table-of-contents block to `word-core`.
- Derive entries from supported top-level Heading 1-3 blocks in the editable first-section projection.
- Generate safe document-local bookmark IDs for headings that need TOC targets.
- Add a local desktop File-menu command to insert or update contents from headings.
- Preserve 900Word-authored TOCs through ODT save/reopen with `word900` metadata while rendering visible text and safe internal links.
- Export TOCs as ordinary text in TXT/basic PDF and as safe fragment links in HTML/print HTML.
- Keep deterministic page numbers, live pagination, automatic external ODT TOC interoperability, DOCX TOCs, and PDF link annotations deferred.

## Sprint 025: Footnotes And Endnotes MVP

Status: complete.

- Add bounded local footnote/endnote metadata to `word-core` with durable inline note references and undo/redo command coverage.
- Preserve note references through the desktop ProseMirror projection as local inline atoms instead of plain text.
- Add compact toolbar controls to insert a footnote or endnote with simple local body entry, plus a Notes sidebar that surfaces stored note bodies.
- Save and reopen 900Word-authored notes through ODT `text:note`, `text:note-citation`, and `text:note-body` elements with bounded `word900` metadata.
- Promote bounded ODT notes with matching safe `word900` metadata into local notes, and degrade missing, unsafe, mismatched, over-limit, duplicate, or unanchored note structures to visible text with generic warnings.
- Export note references and note body text conservatively in TXT, sanitized HTML, print HTML, and basic PDF.
- Keep deterministic pagination, page-bottom footnote placement, PDF note layout fidelity, DOCX notes, cross-reference management, and rich note editing deferred.

## Sprint 026: Autocorrect And Smart Typing MVP

Status: complete.

- Add local disabled-by-default settings for smart typing behavior without adding cloud sync, accounts, telemetry, network access, or document metadata.
- Add typed-input transforms for sentence capitalization, smart quotes, double-hyphen em dashes, and a small allowlisted typo replacement map.
- Add simple `- ` and `1. ` list triggers at the start of an otherwise empty top-level paragraph.
- Keep transforms deterministic, bounded to collapsed typed input, and avoid URL-like tokens.
- Keep imported-document cleanup, rich autocorrect dictionaries, locale-specific quote styles, persistent settings storage, and smart typing while track-changes recording deferred.

## Sprint 027: Expanded Stats Panel MVP

Status: complete.

- Expand the bottom-toolbar Stats toggle into a compact document information panel.
- Show words, selection words, characters with and without spaces, paragraphs, model blocks, estimated pages, and estimated reading time.
- Keep page count and reading time clearly labelled as estimates and avoid deterministic pagination claims.
- Surface lightweight local-first document indicators already available in the model: comments, unresolved comments, track changes status/count, images, embedded assets, footnotes, endnotes, and page size.
- Keep the feature UI-only with no cloud, accounts, telemetry, network behavior, new document metadata, import/export format changes, private paths, filenames, usernames, or hostnames.
