# File Formats

## Native Format

OpenDocument Text (`.odt`) is the native saved format.

## Bootstrap Support

- `.odt`: MVP read/write boundary for generated documents, covering paragraphs, headings, inline marks, safe links, safe paragraph/heading bookmarks, internal fragment links, lists, tables, page breaks, metadata title, named paragraph styles, 900Word-authored paragraph style properties, basic page setup, 900Word-authored direct paragraph/text formatting, 900Word-authored headers/footers/page fields, allowlisted embedded image bytes, and 900Word-authored image presentation metadata.
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
- Sprint 012 exposes desktop editing for table cells that contain paragraphs, headings, or lists. Row/column structural editing, merged cells, table styles, and unsupported nested cell content remain deferred.
- Sprint 013 stores simple section-level header/footer regions in `word-core`, persists 900Word-authored page regions through ODT `styles.xml`, and supports page-number, page-count, and date fields. External header/footer structures beyond the simple paragraph/field subset import with warnings and read-only state instead of silent rewrite. Export renders page numbers and total page count as `1` and date as the document modified date because deterministic pagination remains deferred.
- Sprint 014 adds local image insertion for PNG, JPEG, GIF, and WebP files. Imported images are copied into `document.assets` and saved as embedded ODT package entries; source paths and source filenames are not stored.
- Sprint 015 adds editable image alt text, caption text, alignment, and scale percentage. ODT save/reopen preserves 900Word-authored values with bounded `word900` metadata attributes on image frames. Broader ODT-native image sizing/layout compatibility is not claimed.
- Sprint 016 adds optional bookmark IDs to paragraph and heading blocks. ODT save/reopen preserves safe values with native `text:bookmark` elements inside `text:p` and `text:h`, and preserves internal links as safe `#fragment` text links. Unsafe imported bookmark names and unsafe internal hrefs are stripped with warnings. The editor exposes stable link targets only for blocks that already have safe bookmark IDs.
- Sprint 014 and Sprint 015 HTML and print HTML export include allowlisted in-document image assets as `data:image/...;base64,...` URLs under the offline CSP and render image alt/caption/alignment/scale. The basic PDF exporter remains text-oriented and includes image alt/caption text only.
- Sprint 016 HTML and print HTML export emit safe bookmark IDs as element `id` attributes and preserve safe internal `#fragment` hrefs. The basic PDF exporter remains text-oriented and does not emit active internal link annotations.
- Unsupported ODT elements import with warnings.
- Unsupported or unsafe image references import with warnings instead of remote loading.
- Unsupported image payload types are rejected instead of embedded.
- Unsafe text link schemes and unsafe internal fragments are stripped during import.
- Binary `.doc` and broad `.docx` compatibility remain deferred.

## Deferred Support

- `.docx`: limited import/export after ODT stability.
- `.doc`: deferred until external converter security is documented.
- `.epub`: deferred until PDF and HTML export are stable.
