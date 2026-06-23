# File Format Security

All external files are untrusted.

## Required Controls

- Maximum archive size.
- Maximum expanded size.
- Maximum entry count.
- Maximum path depth.
- Maximum XML depth.
- Maximum image size.
- No absolute paths.
- No parent-directory traversal.
- No symlinks.
- No encrypted package entries.
- No external entities.
- No remote relationships.
- No scripts, macros, embedded executables, or unsafe HTML.

## Implemented ODT Preflight

`word-odf` currently enforces:

- Maximum ZIP package size, entry count, per-entry size, total expanded size, path depth, XML depth, and image entry size.
- Rejection of absolute paths, backslash paths, parent-directory traversal, symlink entries, encrypted entries, executable/script-like entries, invalid or missing first-entry stored ODT mimetype values, and missing `content.xml`.
- Rejection of XML `DOCTYPE` and entity declarations before content import.
- Rejection of embedded image payloads outside the PNG, JPEG, GIF, and WebP magic-byte allowlist.
- Warning-based import for unsupported ODT elements.
- Warning-based stripping of unsafe text links and unsafe or remote image references.

## HTML Import Policy

HTML import must strip scripts, event handlers, unsafe CSS URLs, `javascript:`, unexpected `file:`, unsafe SVG, iframe, object, embed, and remote loads by default.
