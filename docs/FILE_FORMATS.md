# File Formats

## Native Format

OpenDocument Text (`.odt`) is the native saved format.

## Bootstrap Support

- `.odt`: MVP read/write boundary for generated documents, covering paragraphs, headings, inline marks, safe links, lists, tables, page breaks, metadata title, named paragraph styles, basic page setup, and allowlisted embedded image bytes.
- `.txt`: export plain document text to a user-entered `.txt` path.
- `.html`: export offline sanitized semantic HTML to a user-entered `.html` path.
- `.pdf`: export a valid basic PDF byte stream to a user-entered `.pdf` path for smoke testing and simple sharing.

## Current ODT Limits

- ODT is the native saved package format, but full layout fidelity is not claimed.
- Sprint 005 page setup controls serialize and import basic page width, page height, and margin metadata. Full layout fidelity and deterministic pagination are not claimed.
- Sprint 007 print HTML uses page setup metadata for browser/WebView print margins. Deterministic pagination and full layout fidelity remain deferred.
- Sprint 007 PDF export is a simple text-oriented adapter with a generated xref table and no embedded fonts, remote resources, or deterministic layout engine.
- Unsupported ODT elements import with warnings.
- Unsupported or unsafe image references import with warnings instead of remote loading.
- Unsupported image payload types are rejected instead of embedded.
- Unsafe text link schemes are stripped during import.
- Binary `.doc` and broad `.docx` compatibility remain deferred.

## Deferred Support

- `.docx`: limited import/export after ODT stability.
- `.doc`: deferred until external converter security is documented.
- `.epub`: deferred until PDF and HTML export are stable.
