# File Formats

## Native Format

OpenDocument Text (`.odt`) is the native saved format.

## Bootstrap Support

- `.odt`: minimal write/read boundary for generated documents.
- `.txt`: export plain document text.
- `.html`: export sanitized semantic HTML.
- `.pdf`: basic PDF bytes for early smoke tests.

## Deferred Support

- `.docx`: limited import/export after ODT stability.
- `.doc`: deferred until external converter security is documented.
- `.epub`: deferred until PDF and HTML export are stable.
