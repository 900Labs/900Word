# Sprint 077: Security Hardening Audit

## Status

Complete.

## Goal

Close the concrete security and correctness defects found in a full audit of the untrusted file-parsing, export, document-model, spell-check, and desktop layers, without changing the privacy model, the canonical ODT format, the offline-first behavior, or adding new dependencies, telemetry, accounts, cloud behavior, network access, or plugin runtime scope.

## Shipped

- `word-odf` and `word-docx` now enforce package expanded-size limits against the real decompressed byte count on every archive entry read, using a hard `Read::take` cap in both preflight and content-read passes, with real-length total accounting. The declared-size fast reject is retained.
- Removed the redundant `<!doctype`/`<!entity>` substring pre-scan from both importers; real document-type declarations are rejected via the XML parser `DocType` event. Added the compensating `DocType` event handler to DOCX preflight.
- `word-odf` validates PNG, JPEG, and GIF pixel dimensions on import and rejects entries exceeding bounded per-side and total pixel limits.
- `word-export` validates inline font family, text color, and highlight color against strict allowlists before emitting them into HTML/print HTML `style` attributes, caps embedded HTML images with per-image and per-document limits and a figure fallback, and rejects non-ASCII PDF URI annotations.
- `word-core` caps undo/redo history to a bounded number of entries and bounds every recursive block walker with a depth guard.
- `word-spell` indexes suggestions by first character and character length, caps candidate evaluation, and reads dictionaries and personal word lists with a hard byte-counting cap.
- The desktop shell tightens the content security policy, sandboxes the print iframe without script execution, validates editor schema paragraph spacing/indent and list definition attributes, and guards editor-to-backend sync with a load-generation token so document-replacing actions cannot be silently overwritten by a stale sync.

## Compatibility Boundary

- `.odt` remains the canonical saved package format; DOCX remains a conversion-only boundary.
- Expanded-size, entry-count, path-depth, XML-depth, image byte-size, and image pixel-dimension limits are enforced as documented; a package that exceeds any of them is rejected with an existing generic error.
- Strict HTML inline-CSS validation intentionally drops values that are not allowlisted (for example CSS color shorthands or non-generic font names); unsupported inline formatting degrades to unstyled text rather than emitting untrusted CSS.
- HTML and print HTML still escape all document text, allow only `http`, `https`, `mailto`, and safe local `#fragment` links, and embed only allowlisted `data:image/...` URLs.
- Spell-check suggestion ordering remains deterministic; suggestions are unchanged for dictionaries within normal size.
- No local paths, private filenames, usernames, hostnames, account data, document text, telemetry identifiers, network state, or recovery paths are exposed by any change in this sprint.

## Deferred

- DOCX relationship target path-depth validators still resolve against default package limits; this is defense-in-depth only and does not affect the enforced decompression or size caps.
- Full iterative (rather than depth-bounded) block walking, richer image pixel decoding for WebP, and broader office-suite compatibility fidelity remain future work.

## Verification

- `cargo fmt --all -- --check`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets`
- `npm --workspace apps/desktop run check`
- `npm --workspace apps/desktop run test`
- Regression tests cover the decompression-bomb rejection (lying-header packages), benign `DOCTYPE` substring acceptance, oversized image dimension rejection, inline CSS injection rejection, embedded HTML image caps, non-ASCII PDF URI rejection, bounded undo history, deep-nesting stack safety, bounded spell-check candidate evaluation, and the capped dictionary read.

## Privacy Notes

- None of these changes add network calls, telemetry, accounts, cloud sync, remote fetch, external converters, plugin runtime behavior, or new dependencies.
- Existing privacy guarantees are preserved: recent and recovery paths remain backend-only behind opaque tokens; IPC and export errors remain generic; exported and saved files continue to avoid local paths, usernames, hostnames, and private build metadata.
