# Sprint 055: PDF Note Backlinks

## Status

Complete.

## Goal

Add bounded active PDF navigation between local footnote/endnote reference labels and appended note-body rows without adding a page-bottom note layout engine, note continuation fidelity, remote fetching, telemetry, cloud behavior, accounts, or heavy PDF dependencies.

## Shipped

- Extended the PDF linked-text projection so valid local footnote/endnote reference labels link to the generated note-body row for the same note when both endpoints are exported.
- Marked generated note-body rows as internal PDF destinations and made their visible row labels link back to the first exported note reference.
- Preserved existing per-page and per-document PDF annotation caps.
- Omitted note links when either endpoint is outside the selected PDF page range.
- Kept visible note text output compatible with the existing inline reference plus appended Footnotes/Endnotes section behavior.
- Avoided writing note IDs or synthetic destination IDs to generated PDF bytes.

## Implementation Notes

- Note links use generated page-object `/Dest` arrays, not `/URI` actions.
- Synthetic note destination IDs are in-memory only and are not serialized into PDF output.
- Duplicate note references use the first validated reference that matches a stored note of the same kind as the body backlink target.
- Note destinations use typed in-memory destination IDs so document bookmark IDs cannot collide with note-body or note-reference destinations.
- Link rectangles reuse the existing lightweight PDF text-run approximation.

## Review Coverage

- Added regression coverage for note destinations coexisting with legal bookmark IDs that share the old string-prefix shape.
- Added regression coverage for a mismatched duplicate note reference before a later valid same-ID reference.
- Added regression coverage for a valid note reference before a later mismatched same-ID reference.

## Deferred

- Page-bottom footnote placement.
- Note continuation separators and continuation text.
- Rich note body formatting/layout.
- Exact glyph-level link geometry.
- Full office-suite PDF layout compatibility.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-export note`
- `cargo test -p word-export pdf_export`
- `cargo clippy -p word-export -- -D warnings`
- `git diff --check`
- `./scripts/verify-local.sh`
