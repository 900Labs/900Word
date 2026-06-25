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
- Warning-based stripping of unsafe text links, unsafe internal link fragments, unsafe bookmark names, and unsafe or remote image references.

## Implemented DOCX Preflight

`word-docx` currently enforces:

- Maximum ZIP package size, entry count, per-entry size, total expanded size, path depth, and XML depth.
- Rejection of absolute paths, backslash paths, parent-directory traversal, symlink entries, encrypted entries, macro/executable-like entries, ActiveX, embedded object, script, and custom XML package areas.
- Rejection of XML `DOCTYPE` and entity declarations before content import.
- Required `word/document.xml` presence.
- Relationship import limited to safe hyperlink targets, safe local header/footer targets under `word/`, safe local comments targets under `word/`, and safe local image targets under `word/media/`. Unsupported external relationships are ignored with generic warnings and never fetched.
- Header/footer relationship targets are accepted only when they resolve to simple package paths for `word/header*.xml` or `word/footer*.xml` parts that have already passed package preflight. Unsafe, missing, remote, or unsupported targets are ignored with generic warnings.
- Comments relationship targets are accepted only when they resolve to simple package paths for `word/comments.xml` or `word/comments*.xml` parts that have already passed package preflight. Unsafe, missing, remote, over-limit, malformed, unanchored, threaded/reply, or unsupported comment content is ignored with generic warnings and is not stored as hidden metadata.
- Image relationship targets are accepted only when they resolve to simple package paths under `word/media/` with PNG, JPEG/JPG, GIF, or WebP extensions. Image bytes are read only after package preflight and must match the allowlisted magic bytes and declared media type. Unsafe, missing, remote, mismatched, or unsupported targets are ignored with generic warnings.
- No remote media fetch, shell execution, external converter invocation, account lookup, or network behavior.

## Local Image Import Policy

Sprint 014 treats user-selected local images as untrusted input. The Rust desktop command rejects traversal-shaped paths, unsupported extensions, non-regular files, empty files, files over 8 MiB, and files whose magic bytes do not match the allowlisted PNG, JPEG, GIF, or WebP media type. Accepted image bytes are copied into embedded document assets under generated generic asset names. Source paths and source filenames are not stored.

Sprint 015 image presentation metadata is bounded document metadata only: editable alt text, caption text, alignment, and scale percentage. ODT save/reopen preserves 900Word-authored values in the `urn:900labs:900word:metadata` namespace without adding remote image loading, local path references, source filenames, scripts, or executable content.

## Recovery Snapshot Policy

Sprint 029 recovery snapshots use validated opaque local tokens and generic summaries only. Autosave writes owner-only recovery files on Unix, bounds retention to 3 snapshots per document and 20 snapshots overall, and keeps legacy single-file recovery tokens recoverable without writing new snapshots in that shape. Symlinked recovery entries are ignored during listing and rejected during recovery open. Opening a recovery snapshot still passes through the normal ODT size and package validation path, and the recovered draft is dirty and unsaved rather than adopting the recovery file as a save target.

## HTML Import Policy

HTML import must strip scripts, event handlers, unsafe CSS URLs, `javascript:`, unexpected `file:`, unsafe SVG, iframe, object, embed, and remote loads by default.

## Export Policy

Sprint 007 exporters write only to user-entered paths with validated `.txt`, `.html`, or `.pdf` extensions and no traversal components. Sprint 031 adds the same path validation and format/byte-count-only result policy for `.docx` conversion export. Export command results expose only the format and byte length to the frontend.

HTML export is generated from `word-core`, escapes document text, strips unsafe link schemes, allows only `http`, `https`, `mailto`, and safe local `#fragment` text links, does not emit scripts, event handlers, iframe/object/embed content, remote images, local file references, or raw imported HTML, and includes a restrictive offline CSP meta tag. Sprint 014 allows only embedded `data:image/png`, `data:image/jpeg`, `data:image/gif`, and `data:image/webp` URLs generated from in-document asset bytes. Sprint 015 adds escaped image captions and bounded alignment/scale styles generated from document metadata. Sprint 016 emits only sanitized bookmark IDs as HTML `id` attributes. Print HTML uses the same sanitizer and adds page setup CSS for WebView print.

PDF export is a minimal generated document for smoke testing and simple sharing. It contains no local paths, hostnames, usernames, embedded files, remote references, scripts, or macros. Non-ASCII text is degraded in the bootstrap PDF adapter until a font/layout strategy is accepted.

DOCX export is a minimal generated package built from `word-core`. It contains no source local paths, hostnames, usernames, macros, scripts, ActiveX, custom XML, linked images, or remote images. External hyperlinks are emitted as ordinary DOCX hyperlink relationships only when their targets pass the local safe-link allowlist; 900Word does not open or prefetch them. Simple page-region exports generate only local header/footer XML parts and document relationships. Image exports embed only valid bounded allowlisted in-document asset bytes as generated `word/media/` parts with generated relationship targets. Comment exports generate only `word/comments.xml`, a local document relationship, numeric DOCX comment IDs, and simple range/reference markers for valid anchored `word-core` comments.

## Dictionary Input Policy

Hunspell-shaped `.aff` and `.dic` files are treated as untrusted local input. Sprint 006 supports UTF-8 word-list dictionaries only, enforces a per-file size limit for user dictionaries, ignores incomplete user dictionary pairs, validates language-tag filenames, and exposes no local dictionary paths to frontend state. Affix-rule expansion is deferred.
