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
- Keep dedicated alt text editing, image resizing/cropping, and PDF image embedding deferred to later scoped sprints.

## Sprint 015: Image Polish MVP

Status: complete.

- Add durable image presentation metadata to `word-core`: editable alt text, caption text, alignment, and bounded scale percentage.
- Add compact contextual desktop controls for the selected image atom without introducing a custom image editor framework.
- Preserve image metadata through ProseMirror projection/text sync and 900Word-authored ODT save/reopen using the existing `word900` metadata namespace.
- Reflect image alt/caption/alignment/scale in sanitized HTML and print HTML export.
- Keep cropping, drag resize handles, richer external ODT image-layout fidelity, and PDF image embedding deferred to later scoped sprints.

## Sprint 016: Bookmarks And Internal Links MVP

Status: complete.

- Add optional safe bookmark IDs to durable paragraph and heading blocks.
- Preserve bookmark IDs through ProseMirror projection and editor sync.
- Add compact controls to create/remove a bookmark on the selected paragraph or heading and link selected text to an existing bookmark/heading target.
- Preserve 900Word-authored bookmarks and internal links through ODT using `text:bookmark` anchors and `#fragment` text links.
- Emit sanitized HTML element IDs and internal fragment hrefs.
- Keep richer bookmark management, automatic heading ID assignment, cross-document links, and exact glyph-level PDF link geometry deferred. Sprint 054 later adds bounded active internal PDF destinations for safe exported targets.

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
- Keep thread replies, multiple authors, comment search, full external ODT annotation compatibility, DOCX comments, and PDF comment annotation export deferred.

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
- Export TOCs as ordinary text in TXT/basic PDF and as safe fragment links in HTML/print HTML. Sprint 054 later adds bounded active internal PDF destinations for generated TOC entries whose safe targets are exported.
- Keep deterministic page numbers, live pagination, automatic external ODT TOC interoperability, and DOCX TOCs deferred.

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
- Keep imported-document cleanup, rich autocorrect dictionaries, locale-specific quote styles, and smart typing while track-changes recording deferred.

## Sprint 027: Expanded Stats Panel MVP

Status: complete.

- Expand the bottom-toolbar Stats toggle into a compact document information panel.
- Show words, selection words, characters with and without spaces, paragraphs, model blocks, estimated pages, and estimated reading time.
- Keep page count and reading time clearly labelled as estimates and avoid deterministic pagination claims.
- Surface lightweight local-first document indicators already available in the model: comments, unresolved comments, track changes status/count, images, embedded assets, footnotes, endnotes, and page size.
- Keep the feature UI-only with no cloud, accounts, telemetry, network behavior, new document metadata, import/export format changes, private paths, filenames, usernames, or hostnames.

## Sprint 028: Accessibility And Low-Resource Mode MVP

Status: complete.

- Add disabled-by-default local settings for larger toolbar controls, reduced motion, and low-resource mode.
- Surface the controls as keyboard-focusable native checkboxes in Settings.
- Apply the controls through desktop UI classes and data attributes while keeping high contrast independent.
- Make low-resource mode suppress nonessential automatic sidebar content and reduce decorative visual weight while preserving recovery, warnings, and explicit user-opened panels.
- Keep the feature UI-only with no cloud, accounts, telemetry, network behavior, new document metadata, import/export format changes, private paths, filenames, usernames, or hostnames.

## Sprint 029: Recovery Snapshots MVP

Status: complete.

- Replace single-file autosave recovery overwrites with versioned local recovery snapshots.
- Use validated opaque recovery tokens and generic recovery labels without local paths, filenames, usernames, hostnames, source document names, or recovery locations.
- Keep recovery files owner-only on Unix and keep recovered drafts dirty and unsaved.
- Bound recovery retention to 3 snapshots per document and 20 snapshots overall.
- Preserve list, recover, and discard compatibility for legacy single-file recovery tokens while writing only the new versioned snapshot shape.
- Keep the feature local-only with no cloud, accounts, telemetry, network behavior, AI services, encryption claims, import/export changes, or document metadata changes.

## Sprint 030: Document Inspector MVP

Status: complete.

- Add a lightweight Document Inspector reachable from the File menu and bottom toolbar.
- Show canonical `.odt` format, generic saved-location state, saved/unsaved status, created/modified metadata timestamps, page size, core stats, embedded image count/bytes, comments, unresolved comments, track changes status/count, footnotes, and endnotes.
- Add local privacy warnings for comments, tracked changes, document title metadata, recovery drafts, and unsaved state.
- Keep paths, private filenames, source image filenames, usernames, hostnames, recovery locations, and document text out of inspector summaries.
- Keep the feature local-only and frontend-only with no cloud, accounts, telemetry, network behavior, heavy dependencies, import/export changes, or saved metadata changes.

## Sprint 031: DOCX Compatibility MVP

Status: complete.

- Add a lightweight `word-docx` conversion crate using existing ZIP and XML tooling.
- Keep ODT canonical: `.docx` import opens as an unsaved dirty document, and `.docx` export does not change the current save path.
- Support simple DOCX paragraphs, Heading 1-3, bold/italic/underline, safe hyperlinks, basic list fallback, and simple tables.
- Emit generic warnings for degraded or ignored DOCX import content.
- Export minimal valid `.docx` packages with paragraphs, headings, basic inline marks, safe hyperlinks, simple lists, and simple tables.
- Keep cloud, accounts, telemetry, network behavior, external converters, heavy dependencies, macros, media import, comments, tracked changes, headers/footers, rich note fidelity, and full DOCX fidelity deferred.

## Sprint 032: PDF Pagination And Export Settings MVP

Status: complete.

- Add typed PDF export options in `word-export` while preserving the existing compatibility wrapper.
- Generate valid multi-page PDF output with one page object per deterministic lightweight text page.
- Use `word-core` page setup for PDF page size and margins, honor explicit page breaks, and keep text above the bottom margin.
- Repeat simple header/footer text on generated pages and render page-number, page-count, and date fields deterministically.
- Carry existing text projections for paragraphs, headings, lists, tables, TOCs, notes, and image alt/caption text into the PDF pagination path.
- Add PDF page-range export settings in the desktop File > Export flow with backend validation and generic range errors.
- Keep raster image embedding, embedded/subset fonts, exact glyph-level PDF link geometry, complex script shaping, page-bottom note layout, and editor-preview layout fidelity deferred. Sprint 054 later adds bounded internal PDF destinations without a layout engine.

## Sprint 033: DOCX Page Regions

Status: complete.

- Extend the conversion-only DOCX boundary to import default and first-page header/footer relationships into `word-core` page regions.
- Read only safe relationship-resolved `word/header*.xml` and `word/footer*.xml` package parts that pass existing preflight, and keep warnings generic.
- Map simple paragraph runs and supported page-number, page-count, and date fields through the existing `PageRegions` model.
- Export 900Word-authored page regions as minimal DOCX header/footer parts with document relationships and section references.
- Keep ODT canonical, with DOCX images/media, comments, tracked changes, rich note fidelity, even-page regions, complex fields, complex section layouts, cloud, telemetry, accounts, network behavior, and full layout fidelity deferred.

## Sprint 034: DOCX Image Media

Status: complete.

- Extend the conversion-only DOCX boundary to import safe local image relationships under `word/media/` into existing `word-core` assets and `ImageBlock`s.
- Validate supported PNG, JPEG/JPG, GIF, and WebP image payloads by relationship target, extension/media type, package preflight, and magic bytes before storing bytes.
- Preserve adjacent visible paragraph text around simple DOCX drawings by splitting mixed text/image paragraphs into adjacent text and image blocks when needed.
- Export valid 900Word-authored image assets as generated embedded `word/media/` parts with document relationships, content type defaults, minimal DrawingML references, and alt text.
- Keep ODT canonical, with linked/remote images, image sizing/cropping/layout fidelity, compression/downsampling, comments, tracked changes, rich note fidelity, cloud, telemetry, accounts, network behavior, external converters, heavyweight dependencies, and full DOCX media fidelity deferred.

## Sprint 035: DOCX Comments MVP

Status: complete.

- Extend the conversion-only DOCX boundary to import simple anchored legacy comments into existing `word-core` `CommentThread`s and `Inline.comment_ids`.
- Read only safe relationship-resolved `word/comments.xml` or `word/comments*.xml` package parts that pass existing preflight, and keep warnings generic.
- Export valid 900Word-authored anchored comments as generated `word/comments.xml`, a generated document relationship, a content-type override, and simple range/reference markers around supported inline text.
- Keep ODT canonical, with DOCX replies, threaded comments, resolved-state fidelity, tracked changes, rich note fidelity, full review fidelity, cloud, telemetry, accounts, network behavior, external converters, heavyweight dependencies, and full DOCX comments fidelity deferred.

## Sprint 036: PDF Table And Image Rendering

Status: complete.

- Extend the lightweight PDF export body projection with text lines, simple table rows/cells, and image figure placeholders.
- Render simple PDF tables as vector cell boxes with wrapped cell text while preserving generated page objects and page ranges.
- Render image blocks as bounded visible figure placeholders using alt text, captions, alignment, and scale metadata without embedding raster image bytes or emitting asset IDs, source names, local paths, usernames, or hostnames as image metadata.
- Keep ODT canonical. Sprint 038 later adds bounded JPEG embedding and Sprint 054 later adds bounded internal destinations; PNG/GIF/WebP PDF embedding, merged cells, table resizing, rich table styling, formulas, complex nested table layout, embedded/subset fonts, exact glyph-level PDF link geometry, complex script shaping, and full layout fidelity remain deferred.

## Sprint 037: PDF Link Annotations

Status: complete.

- Extend the lightweight PDF export body projection with safe external URI link spans for paragraph, heading, list, and table cell text.
- Emit bounded active PDF `/Link` annotations using `/A << /S /URI /URI (...) >>` for safe `http`, `https`, and `mailto` links only.
- Place annotations over approximate text-run rectangles from the existing lightweight line layout without creating page-wide clickable areas.
- Degrade over-budget links to visible rendered text without annotation objects using per-page and per-export caps.
- Keep ODT canonical and keep exact glyph-level link geometry, PDF comment/note annotations, remote fetching, telemetry, accounts, cloud behavior, and full layout fidelity deferred. Sprint 054 later adds bounded active internal PDF destinations for safe exported bookmark targets.

## Sprint 038: PDF JPEG Image Embedding

Status: complete.

- Embed safe in-document baseline JPEG/JPG assets in generated PDFs as bounded `/DCTDecode` image XObjects after APP/COM JPEG metadata marker stripping.
- Keep PNG, GIF, WebP, malformed JPEG, oversized JPEG, unsupported component-count, post-scan metadata-marker, and over-cap cases on the visible figure-placeholder fallback path with alt/caption text when present.
- Bound JPEG embedding to 32 images per generated PDF, 8 MiB per embedded JPEG, 8192 px per side, 20,000,000 pixels, and grayscale/RGB component counts.
- Preserve page size, margins, image alignment, bounded scale metadata, and page-range selection behavior without adding a heavy PDF or image-processing dependency.
- Keep ODT canonical and keep PNG/GIF/WebP PDF embedding, progressive JPEG embedding, JPEG decoding, downsampling/recompression, crop/rotation, EXIF interpretation/selective preservation, color-management precision, rich PDF image metadata, remote fetching, telemetry, accounts, cloud behavior, and full layout fidelity deferred.

## Sprint 039: DOCX Tracked Changes MVP

Status: complete.

- Extend the conversion-only DOCX boundary to import simple `w:ins` and `w:del` text revisions in body paragraphs, list items, and table cells into existing `word-core` inline tracked changes.
- Generate safe local tracked-change IDs on import, sanitize bounded author metadata, and use safe deterministic fallback metadata when DOCX revision authors or dates are unsafe.
- Export 900Word-authored text-only insertions/deletions as simple generated `w:ins` and `w:del` / `w:delText` markup with numeric DOCX revision IDs.
- Keep ODT canonical and keep formatting-only changes, table/image changes, move changes, resolved state, compare/merge, full Word review fidelity, telemetry, accounts, cloud behavior, external converters, heavyweight dependencies, and full DOCX fidelity deferred.

## Sprint 040: DOCX Footnotes/Endnotes MVP

Status: complete.

- Extend the conversion-only DOCX boundary to import simple relationship-resolved `word/footnotes.xml` and `word/endnotes.xml` note parts into existing `word-core` notes.
- Generate safe local note IDs on import, use bounded visible numeric labels, and keep unsupported or hidden DOCX note metadata on generic warning/fallback paths.
- Export supported 900Word-authored local footnotes/endnotes as generated DOCX note parts with generated relationship IDs, content type overrides, numeric note IDs, and simple body paragraphs.
- Keep ODT canonical and keep page-bottom placement, continuation separator fidelity, note cross references, rich note formatting/layout, comments/tracked changes inside note bodies, full Word note fidelity, telemetry, accounts, cloud behavior, external converters, heavyweight dependencies, and full DOCX fidelity deferred.

## Sprint 041: Editor Toolbar Selection Fixes

Status: complete.

- Preserve saved editor selections for core toolbar formatting commands when mouse or pointer activation would otherwise move focus to the toolbar.
- Cover inline mark buttons and paragraph/heading buttons through the same saved-selection command path.
- Keep the fix local to editor interaction with no import/export, metadata, telemetry, network, accounts, cloud, or document-content logging changes.

## Sprint 042: Dictionary Manager MVP

Status: complete.

- Add an offline dictionary manager section under Settings using the existing local dictionary list and settings commands.
- Let users select the active spell-check dictionary from installed bundled/user dictionaries and save it through the existing settings flow.
- Show installed dictionary display name, language tag, bundled/user source type, license, and generic no-local-path source label.
- Show explicit offline/local-only status and a graceful fallback message when the selected dictionary is unavailable.
- Keep downloads, remote dictionary lookup, cloud sync, telemetry, accounts, plugin runtime behavior, and dictionary install/import UX deferred.

## Sprint 043: Persistent Settings MVP

Status: complete.

- Persist sanitized local app settings under the existing app-data boundary using private atomic writes where supported.
- Force telemetry off on every settings save/load and normalize language/UI locale values before storing or returning settings.
- Fall back to sanitized defaults when the settings file is missing, malformed, unreadable, path-shaped, oversized, or otherwise unsafe.
- Keep document paths, recent paths, filenames, document text, usernames, hostnames, account/cloud identity, telemetry identifiers, network state, cloud sync, plugin runtime behavior, import/export, and UI behavior changes out of scope.

## Sprint 044: Local Settings Reset MVP

Status: complete.

- Add a Settings reset command and button that restore sanitized local defaults through the existing app-data settings path.
- Rewrite the local settings file with private atomic writes instead of deleting it, preserving owner-only permissions where supported.
- Return telemetry-disabled defaults to the frontend and show localized English/Spanish reset status text.
- Keep settings import/export, cloud sync, accounts, telemetry, network behavior, plugin-managed settings, document metadata, recent-document persistence, and recovery locations out of scope.

## Sprint 045: Personal Dictionary Manager MVP

Status: complete.

- Add local-only personal dictionary word listing and per-word removal for the active spell-check dictionary.
- Return plain normalized words only and keep local personal dictionary paths, filenames, usernames, hostnames, account/cloud identity, telemetry identifiers, network state, recent paths, recovery locations, and document text out of frontend state and errors.
- Reuse the existing app-data user dictionary root validation and owner-only local file permissions where supported.
- Add a compact Settings personal dictionary section with refresh, remove, empty, and generic unavailable states.
- Keep dictionary import/export, downloads, remote lookup, cloud sync, accounts, telemetry, network behavior, plugin-managed dictionaries, and document metadata changes out of scope.

## Sprint 046: Local Hunspell Dictionary Install MVP

Status: complete.

- Add compact Settings controls to choose local `.aff` and `.dic` Hunspell files through native dialogs and install them into the existing app-data dictionary folder.
- Validate language tags and source files in Rust, copy accepted pairs to normalized `<language-tag>.aff` and `<language-tag>.dic` filenames, and validate the copied pair before listing it.
- Refresh installed dictionaries, select the installed language, refresh personal dictionary words, and show generic success/failure statuses without rendering selected local paths or filenames.
- Keep downloads, remote lookup, cloud sync, accounts, telemetry, plugin-managed dictionaries, document metadata changes, and rich install diagnostics that expose private file details out of scope.

## Sprint 047: Local User Dictionary Removal MVP

Status: complete.

- Add compact Settings remove actions for installed local user dictionaries while hiding removal for bundled dictionaries.
- Remove only backend-computed app-data Hunspell pair files for a validated language tag, including the existing underscore alias pair, without accepting arbitrary deletion paths from the frontend.
- Leave personal dictionary word lists intact, refresh dictionaries and personal words after removal, and fall back to bundled English or the first available dictionary when the active user dictionary is removed.
- Keep downloads, remote lookup, cloud sync, accounts, telemetry, plugin-managed dictionaries, bundled dictionary deletion, document metadata changes, and rich removal diagnostics that expose private file details out of scope.

## Sprint 048: Table Cell Styling MVP

Status: complete.

- Add durable `word-core` table-cell presentation metadata for bounded light background choices, optional per-cell text alignment, and border visible/hidden state.
- Add ProseMirror projection, schema validation, sync, and compact contextual toolbar controls for supported editable table cells.
- Preserve 900Word-authored metadata through ODT save/reopen with bounded `word900` attributes.
- Reflect supported cell background, alignment, and border visibility in sanitized HTML/print HTML and the lightweight PDF table projection where practical.
- Keep merged cells, formulas, cell sizing, rich table themes, arbitrary colors, per-side borders, external table-style compatibility claims, network behavior, telemetry, accounts, and cloud sync out of scope.

## Sprint 049: Plain-Text Tabular Paste MVP

Status: complete.

- Detect simple tab-separated plain text on the existing multiline paste path and insert a supported editable table when the selection is an empty top-level block or full top-level content.
- Bound pasted tables to the existing 1-8 row and 1-8 column table limits, require at least two rows and one tab, and normalize CRLF/CR line endings.
- Pad simple one-cell-short rows with editable empty cells while falling back to existing paragraph paste for out-of-bounds or too-irregular tabular text.
- Keep list paste priority, native partial replacement paste behavior, `.odt` as canonical saved format, and `word-core` table sync unchanged.
- Keep rich spreadsheet paste, formulas, merged cells, table sizing, HTML clipboard import, external spreadsheet parser dependencies, network behavior, telemetry, accounts, and cloud sync out of scope.

## Sprint 050: JPEG Import Metadata Stripping MVP

Status: complete.

- Strip APP0-APP15 and COM marker segments from accepted local JPEG/JPG image imports before embedding the bytes in `word-core` document assets.
- Keep SOI/EOI, structural image segments, scan headers, and entropy-coded image data intact when the JPEG marker structure is accepted.
- Reject malformed or ambiguous metadata-bearing JPEGs with the existing generic unsupported-image error instead of storing partially rewritten bytes.
- Preserve PNG, GIF, and WebP import behavior, `.odt` as canonical saved format, source path/filename omission, and the existing 8 MiB import limit.
- Keep JPEG decoding, EXIF interpretation/selective preservation, compression, downsampling, resizing, crop/rotation, broad malformed-JPEG recovery, network behavior, telemetry, accounts, cloud sync, and heavy image-processing dependencies out of scope.

## Sprint 051: User Template Library MVP

Status: complete.

- Add app-data-scoped local user templates stored as private ODT bytes plus private minimal metadata where supported.
- Let users save the current document as a sanitized-name user template, list user templates together with generated templates, create a new unsaved clean document from a user template, and delete only user templates.
- Use generated opaque user-template IDs and reject traversal, plain paths, symlinks, non-regular files, malformed ODT packages, and oversized template files with generic errors.
- Keep generated template IDs stable and undeletable, with source/description summaries that do not include local paths, filenames, document text, usernames, hostnames, account/cloud identifiers, or telemetry identifiers.
- Keep downloadable template packs, network catalogs, accounts, cloud sync, telemetry, remote assets, template previews, and richer browsing deferred.

## Sprint 052: Table Column Widths MVP

Status: complete.

- Add durable `word-core` table column width hints as bounded per-mille integers for rectangular editable tables with 1-8 columns.
- Preserve valid width hints through ProseMirror schema/projection and keep add/delete column operations metadata-consistent.
- Add compact contextual selected-column width controls using the existing selection-preserving toolbar command path.
- Preserve 900Word-authored widths through ODT `word900` table metadata and safely ignore invalid external values.
- Reflect valid hints in sanitized HTML/print HTML colgroups and proportional lightweight PDF table layout where practical.
- Keep merged cells, formulas, arbitrary CSS, rich table themes, spreadsheet import, remote behavior, telemetry, accounts, cloud sync, heavy dependencies, and full table layout fidelity deferred.

## Sprint 053: DOCX Table Width Interoperability

Status: complete.

- Import simple DOCX `w:tblGrid` / `w:gridCol w:w` width hints for editable rectangular tables with 1-8 columns.
- Normalize positive DOCX grid widths into bounded per-mille `word-core` `Table.column_widths` values through the existing sanitizer.
- Export valid 900Word-authored `Table.column_widths` as generated DOCX table grid hints.
- Ignore invalid, mismatched, duplicate, zero, overflowed, missing, merged-cell, nested-table, or unsupported DOCX width metadata without preserving path-like or source-identifying values.
- Keep ODT canonical and keep full table layout fidelity, drag resizing, merged cells, formulas, arbitrary CSS, telemetry, network behavior, accounts, cloud sync, and heavy dependencies deferred.

## Sprint 054: PDF Internal Destinations

Status: complete.

- Extend the existing lightweight PDF linked-text projection to retain safe internal `#bookmark` targets alongside safe external URI targets.
- Collect unique safe paragraph/heading bookmark destinations from generated PDF page positions and omit duplicate, missing, unsafe, or page-range-excluded targets.
- Emit bounded PDF `/Link` annotations with `/Dest` arrays for safe internal text links and generated TOC entries whose target exists in exported PDF content.
- Preserve existing per-page and per-document annotation caps and approximate text-run rectangles without creating page-wide clickable areas.
- Keep ODT canonical and keep full PDF outline/bookmark trees, exact glyph-level geometry, remote fetching, telemetry, accounts, cloud behavior, heavy PDF dependencies, and full layout fidelity deferred.

## Sprint 055: PDF Note Backlinks

Status: complete.

- Extend the lightweight PDF linked-text projection to turn valid local footnote/endnote reference labels into internal destination links to appended note-body rows.
- Mark generated note-body rows as PDF destinations and make their visible row labels link back to the first exported note reference when that reference is present in the selected page range.
- Preserve existing per-page and per-document annotation caps and omit links when either endpoint is outside the selected PDF pages.
- Avoid writing note IDs, raw internal target strings, paths, filenames, usernames, hostnames, telemetry identifiers, account/cloud identifiers, or network state to PDF bytes or logs.
- Keep ODT canonical and keep page-bottom footnote layout, note continuation fidelity, rich note formatting/layout, exact glyph-level geometry, remote fetching, telemetry, accounts, cloud behavior, heavy PDF dependencies, and full layout fidelity deferred.

## Sprint 056: DOCX Table Of Contents Interoperability

Status: complete.

- Export 900Word-authored generated TOCs to DOCX as visible styled paragraphs with safe internal bookmark hyperlinks when targets are unique and exported.
- Import the generated `Word900Toc*` style plus safe internal-link shape back into `word-core` `TableOfContents` blocks.
- Preserve unique safe paragraph/heading bookmark IDs through minimal DOCX `bookmarkStart`/`bookmarkEnd` markers.
- Degrade generated-style TOC rows without safe internal links back to visible paragraphs instead of hidden or lossy structured content.
- Keep ODT canonical and keep Word TOC field codes, page-number fidelity, automatic refresh, complex cross references, telemetry, network behavior, accounts, cloud sync, and full DOCX layout fidelity deferred.

## Sprint 057: DOCX Table Cell Presentation Interoperability

Status: complete.

- Import safe DOCX table cell `w:shd` fills into the existing bounded 900Word light color palette.
- Import all-four-sides hidden DOCX cell borders into the existing visible/hidden table cell border model.
- Derive cell text alignment from simple matching paragraph `w:jc` values inside supported table cells.
- Export 900Word-authored safe cell fills, hidden-border markers, and simple cell text alignment into generated DOCX table cells.
- Keep ODT canonical and keep arbitrary colors, partial/per-side borders, rich table themes, merged cells, formulas, deterministic table layout, telemetry, network behavior, accounts, cloud sync, and heavy dependencies deferred.

## Sprint 058: DOCX Paragraph Formatting Interoperability

Status: complete.

- Import simple DOCX paragraph `w:pPr` alignment, automatic line spacing, spacing before/after, start/end indents, and first-line or hanging indents into the existing bounded `word-core` paragraph format model.
- Export valid 900Word-authored paragraph formatting as generated DOCX `w:pPr` spacing, indent, and alignment tags.
- Ignore unsupported line rules, out-of-bounds values, complex Word style inheritance, tabs, borders, shading, outline/keep flags, arbitrary paragraph settings, and full layout fidelity.
- Keep ODT canonical and keep telemetry, network behavior, accounts, cloud sync, rich Word styles, and heavy dependencies deferred.

## Sprint 059: DOCX Inline Formatting Interoperability

Status: complete.

- Import simple DOCX direct run formatting into the existing bounded `word-core` inline mark and inline style model: strikethrough, superscript, subscript, supported menu font sizes, safe direct text colors, and safe highlight colors.
- Export valid 900Word-authored inline formatting as generated DOCX `w:rPr` tags for the same bounded subset.
- Ignore unsupported font sizes, theme colors, automatic colors, arbitrary highlight names, Word font family inheritance, character styles, complex script font variants, and broad run-style fidelity.
- Keep ODT canonical and keep telemetry, network behavior, accounts, cloud sync, rich Word styles, and heavy dependencies deferred.

## Sprint 060: DOCX Page Setup Interoperability

Status: complete.

- Import simple complete body-level section DOCX `w:pgSz` width/height and `w:pgMar` top/right/bottom/left values into the existing bounded `word-core` page setup model when the normalized values validate.
- Export valid 900Word-authored page setup as generated DOCX section `w:pgSz` and `w:pgMar` tags.
- Ignore invalid dimensions, invalid margins, header/footer distances, gutter, columns, multi-section layout, and orientation semantics beyond explicit width/height.
- Keep ODT canonical and keep deterministic pagination, rich section layout fidelity, telemetry, network behavior, accounts, cloud sync, and heavy dependencies deferred.

## Sprint 061: DOCX Explicit Page Break Interoperability

Status: complete.

- Import explicit top-level body paragraph DOCX `w:br w:type="page"` runs into local `PageBreak` blocks.
- Keep ordinary DOCX line breaks, tracked-change page breaks, and table-cell page breaks as visible inline spacing.
- Export 900Word-authored `PageBreak` blocks as generated DOCX `w:br w:type="page"` runs.
- Keep layout-generated page markers, column-break semantics, page-break paragraph flags, deterministic pagination, telemetry, network behavior, accounts, cloud sync, and heavy dependencies deferred.

## Sprint 062: DOCX Paragraph Page-Break-Before Interoperability

Status: complete.

- Import truthy top-level body paragraph DOCX `w:pageBreakBefore` flags into local `PageBreak` blocks before the affected paragraph.
- Ignore falsy `w:pageBreakBefore` values and nested paragraph flags such as table-cell page-break-before with generic warnings.
- Keep export on the existing generated explicit `w:br w:type="page"` run shape for local `PageBreak` blocks.
- Keep hidden Word layout fidelity, deterministic pagination, complex section semantics, telemetry, network behavior, accounts, cloud sync, and heavy dependencies deferred.

## Sprint 063: DOCX Generic Font-Family Interoperability

Status: complete.

- Import allowlisted DOCX `w:rFonts` values into existing generic 900Word inline font-family IDs: `system-ui`, `serif`, `sans-serif`, and `monospace`.
- Export valid 900Word-authored generic inline font families as generated static DOCX `w:rFonts` values.
- Ignore arbitrary source font names, embedded fonts, theme fonts, complex script font attributes, exact font fidelity, telemetry, network behavior, accounts, cloud sync, and heavy dependencies.

## Sprint 064: DOCX Run Shading Interoperability

Status: complete.

- Import safe direct DOCX `w:shd` run fills into the existing bounded 900Word highlight palette.
- Ignore pattern fills, theme fills, arbitrary colors, and broader Word run-style metadata.
- Keep export on generated local `w:highlight` tags for 900Word-authored highlights rather than preserving source shading form.
- Keep ODT canonical and keep full Word style fidelity, telemetry, network behavior, accounts, cloud sync, and heavy dependencies deferred.

## Sprint 065: DOCX Image Scale Interoperability

Status: complete.

- Import bounded square DOCX `wp:extent` image dimensions into the existing 900Word image scale metadata.
- Export valid 900Word-authored image scale metadata as generated square DOCX `wp:extent` and DrawingML extent values.
- Ignore non-square extents, out-of-bounds extents, cropping, rotation, compression/downsampling metadata, arbitrary image sizing, telemetry, network behavior, accounts, cloud sync, and heavy dependencies.
- Keep ODT canonical and keep full DOCX media layout fidelity deferred.

## Sprint 066: DOCX Generated Image Caption Interoperability

Status: complete.

- Export 900Word-authored image captions as visible generated `Word900ImageCaption` DOCX paragraphs after supported image blocks.
- Import plain generated `Word900ImageCaption` paragraphs back into existing 900Word image caption metadata only when they immediately follow a supported image block.
- Keep unstyled, orphaned, linked, rich, or otherwise unsupported caption-like DOCX paragraphs as visible document paragraphs instead of hidden image metadata.
- Keep Word-native caption fields, rich caption formatting, arbitrary media layout fidelity, telemetry, network behavior, accounts, cloud sync, and heavy dependencies deferred.
