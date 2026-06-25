# File Formats

## Native Format

OpenDocument Text (`.odt`) is the native saved format.

## Bootstrap Support

- `.odt`: MVP read/write boundary for generated documents, covering paragraphs, headings, inline marks, safe links, safe paragraph/heading bookmarks, internal fragment links, generated table-of-contents blocks, footnote/endnote references and bodies authored by 900Word, lists, tables, page breaks, metadata title, named paragraph styles, 900Word-authored paragraph style properties, basic page setup, 900Word-authored direct paragraph/text formatting, 900Word-authored headers/footers/page fields, allowlisted embedded image bytes, 900Word-authored image presentation metadata, 900Word-authored comments, and 900Word-authored text-only tracked changes.
- `.txt`: export plain document text to a user-entered `.txt` path.
- `.html`: export offline sanitized semantic HTML to a user-entered `.html` path.
- `.pdf`: export a valid basic PDF byte stream to a user-entered `.pdf` path for smoke testing and simple sharing.

## Current ODT Limits

- ODT is the native saved package format, but full layout fidelity is not claimed.
- Sprint 005 page setup controls serialize and import basic page width, page height, and margin metadata. Full layout fidelity and deterministic pagination are not claimed.
- Sprint 007 print HTML uses page setup metadata for browser/WebView print margins. Deterministic pagination and full layout fidelity remain deferred.
- Sprint 007 PDF export is a simple text-oriented adapter with a generated xref table and no embedded fonts, remote resources, or deterministic layout engine.
- Sprint 009 preserves 900Word-authored paragraph alignment, line spacing, paragraph spacing, indents, inline font family, font size, text color, highlight color, and list levels through generated automatic ODT style names. This is not yet a claim of full external ODT style compatibility.
- Sprint 010 preserves 900Word-authored update-style-from-selection paragraph style properties through generated ODT paragraph styles. This is limited to paragraph formatting properties and is not a full external ODT style editor.
- Sprint 011 exposes desktop hyperlink authoring for the safe external link subset. Sprint 016 extends text links to safe internal fragments. Links are limited to `http`, `https`, `mailto`, and `#bookmark-id` values whose fragment is a compact generated-safe bookmark ID.
- Sprint 012 exposes desktop editing for table cells that contain paragraphs, headings, or lists. Sprint 017 adds bounded 900Word-authored table insertion plus row/column add/delete and delete-table controls for rectangular editable tables. Merged cells, resizing, formulas, heavy table styles, rich table paste, and unsupported nested cell content remain deferred.
- Sprint 013 stores simple section-level header/footer regions in `word-core`, persists 900Word-authored page regions through ODT `styles.xml`, and supports page-number, page-count, and date fields. External header/footer structures beyond the simple paragraph/field subset import with warnings and read-only state instead of silent rewrite. Export renders page numbers and total page count as `1` and date as the document modified date because deterministic pagination remains deferred.
- Sprint 014 adds local image insertion for PNG, JPEG, GIF, and WebP files. Imported images are copied into `document.assets` and saved as embedded ODT package entries; source paths and source filenames are not stored.
- Sprint 015 adds editable image alt text, caption text, alignment, and scale percentage. Sprint 018 adds a direct selected-image resize handle that updates the same bounded scale percentage. ODT save/reopen preserves 900Word-authored values with bounded `word900` metadata attributes on image frames. Broader ODT-native image sizing/layout compatibility is not claimed.
- Sprint 016 adds optional bookmark IDs to paragraph and heading blocks. ODT save/reopen preserves safe values with native `text:bookmark` elements inside `text:p` and `text:h`, and preserves internal links as safe `#fragment` text links. Unsafe imported bookmark names and unsafe internal hrefs are stripped with warnings. The editor exposes stable link targets only for blocks that already have safe bookmark IDs.
- Sprint 020 adds Draft/Page Layout view modes, Fit Width/100%/custom zoom controls, and simple visual rulers in the desktop editor viewport. These settings are local component state only, are not written to `.odt`, and do not add deterministic pagination, page-break preview, font metrics, or print layout fidelity claims.
- Sprint 022 adds bounded local comment threads anchored to selected text through inline comment IDs. ODT save/reopen uses ODF `office:annotation` and `office:annotation-end` elements for 900Word-authored comments, with `word900` metadata for local comment ID and resolved state. Unsafe or invalid external annotations are ignored with generic warnings. Full external ODT annotation compatibility, replies, multi-author identity, DOCX comments, and PDF annotation export are not claimed.
- Sprint 023 adds text-only tracked changes for 900Word-authored insertions and selected-text deletions. ODT save/reopen preserves local change ID, kind, author, timestamp, and recording state with `word900` metadata on inline spans. This keeps deleted text in the package until accepted or rejected. Formatting-only changes, table structure changes, image changes, DOCX track changes, compare/merge, multi-author collaboration, and full external ODT change-tracking fidelity are not claimed.
- Sprint 024 adds 900Word-authored generated table-of-contents blocks. Entries are derived from supported top-level Heading 1-3 blocks in the editable first-section projection and store heading text, level, and a safe internal bookmark target. ODT save/reopen preserves the TOC through `word900:block-type="table-of-contents"` metadata on a visible generated paragraph. External ODT applications may treat it as ordinary text; full ODT-native TOC interoperability, automatic external refresh, deterministic page numbers, and page-number fidelity are not claimed.
- Sprint 025 adds bounded local footnotes and endnotes. ODT save/reopen writes 900Word-authored notes with ODF `text:note`, `text:note-citation`, and `text:note-body` elements plus bounded `word900` note ID/kind metadata. Bounded ODT notes with matching safe `word900` metadata import as local notes and are surfaced in the desktop Notes sidebar. Notes with missing, mismatched, unsafe, duplicate, over-limit, invalid, or unanchored structure import with generic warnings and visible fallback text instead of trusted hidden metadata. Full external ODT note interoperability, cross-reference management, note continuation, deterministic page-bottom placement, and pagination fidelity are not claimed.
- Sprint 026 smart typing is editor typed-input behavior only. It does not add ODT metadata, silently rewrite imported documents, or change import/export compatibility claims. Any transformed text is saved as ordinary document text after the user types it.
- Sprint 014 and Sprint 015 HTML and print HTML export include allowlisted in-document image assets as `data:image/...;base64,...` URLs under the offline CSP and render image alt/caption/alignment/scale. The basic PDF exporter remains text-oriented and includes image alt/caption text only.
- Sprint 016 HTML and print HTML export emit safe bookmark IDs as element `id` attributes and preserve safe internal `#fragment` hrefs. The basic PDF exporter remains text-oriented and does not emit active internal link annotations.
- Sprint 022 TXT, HTML, print HTML, and basic PDF export do not claim comment fidelity. Comment text, sidebars, and active annotations are not exported as PDF/DOCX comments in this MVP.
- Sprint 023 TXT, HTML, print HTML, and basic PDF export do not claim track-changes fidelity. Pending inserted/deleted text may appear as normal document text in simple exports; active DOCX/PDF review metadata is not emitted in this MVP.
- Sprint 024 TXT and basic PDF export render TOCs as ordinary visible text. HTML and print HTML render TOCs as local `<nav>` blocks with safe internal fragment links where targets are safe. No export path emits or claims deterministic TOC page numbers.
- Sprint 025 TXT and basic PDF export render note references inline and append Footnotes/Endnotes sections as ordinary text. HTML and print HTML render local note references and append sanitized note body sections. No export path emits active PDF annotations, deterministic page-bottom placement, or note layout fidelity claims.
- Unsupported ODT elements import with warnings.
- Unsupported or unsafe image references import with warnings instead of remote loading.
- Unsupported image payload types are rejected instead of embedded.
- Unsafe text link schemes and unsafe internal fragments are stripped during import.
- Binary `.doc` and broad `.docx` compatibility remain deferred.

## Deferred Support

- `.docx`: limited import/export after ODT stability.
- `.doc`: deferred until external converter security is documented.
- `.epub`: deferred until PDF and HTML export are stable.
