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

## Local Image Import Policy

Sprint 014 treats user-selected local images as untrusted input. The Rust desktop command rejects traversal-shaped paths, unsupported extensions, non-regular files, empty files, files over 8 MiB, and files whose magic bytes do not match the allowlisted PNG, JPEG, GIF, or WebP media type. Accepted image bytes are copied into embedded document assets under generated generic asset names. Source paths and source filenames are not stored.

## HTML Import Policy

HTML import must strip scripts, event handlers, unsafe CSS URLs, `javascript:`, unexpected `file:`, unsafe SVG, iframe, object, embed, and remote loads by default.

## Export Policy

Sprint 007 exporters write only to user-entered paths with validated `.txt`, `.html`, or `.pdf` extensions and no traversal components. Export command results expose only the format and byte length to the frontend.

HTML export is generated from `word-core`, escapes document text, strips unsafe link schemes, does not emit scripts, event handlers, iframe/object/embed content, remote images, local file references, or raw imported HTML, and includes a restrictive offline CSP meta tag. Sprint 014 allows only embedded `data:image/png`, `data:image/jpeg`, `data:image/gif`, and `data:image/webp` URLs generated from in-document asset bytes. Print HTML uses the same sanitizer and adds page setup CSS for WebView print.

PDF export is a minimal generated document for smoke testing and simple sharing. It contains no local paths, hostnames, usernames, embedded files, remote references, scripts, or macros. Non-ASCII text is degraded in the bootstrap PDF adapter until a font/layout strategy is accepted.

## Dictionary Input Policy

Hunspell-shaped `.aff` and `.dic` files are treated as untrusted local input. Sprint 006 supports UTF-8 word-list dictionaries only, enforces a per-file size limit for user dictionaries, ignores incomplete user dictionary pairs, validates language-tag filenames, and exposes no local dictionary paths to frontend state. Affix-rule expansion is deferred.
