# File Formats

## Native Format

OpenDocument Text (`.odt`) is the native saved format.

## Bootstrap Support

- `.odt`: MVP read/write boundary for generated documents, covering paragraphs, headings, inline marks, safe links, lists, tables, page breaks, metadata title, named paragraph styles, and allowlisted embedded image bytes.
- `.txt`: export plain document text.
- `.html`: export sanitized semantic HTML.
- `.pdf`: basic PDF bytes for early smoke tests.

## Current ODT Limits

- ODT is the native saved package format, but full layout fidelity is not claimed.
- Unsupported ODT elements import with warnings.
- Unsupported or unsafe image references import with warnings instead of remote loading.
- Unsupported image payload types are rejected instead of embedded.
- Unsafe text link schemes are stripped during import.
- Binary `.doc` and broad `.docx` compatibility remain deferred.

## Deferred Support

- `.docx`: limited import/export after ODT stability.
- `.doc`: deferred until external converter security is documented.
- `.epub`: deferred until PDF and HTML export are stable.
