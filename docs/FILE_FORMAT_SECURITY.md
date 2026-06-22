# File Format Security

All external files are untrusted.

## Required Controls

- Maximum archive size.
- Maximum expanded size.
- Maximum entry count.
- Maximum XML depth.
- Maximum image size.
- No absolute paths.
- No parent-directory traversal.
- No symlinks.
- No external entities.
- No remote relationships.
- No scripts, macros, embedded executables, or unsafe HTML.

## HTML Import Policy

HTML import must strip scripts, event handlers, unsafe CSS URLs, `javascript:`, unexpected `file:`, unsafe SVG, iframe, object, embed, and remote loads by default.
