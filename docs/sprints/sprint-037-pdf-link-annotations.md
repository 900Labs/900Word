# Sprint 037: PDF Link Annotations

## Status

Complete.

## Goal

Make exported PDF text links clickable where the lightweight writer can do so safely, while keeping ODT canonical and avoiding a heavy layout engine.

## Shipped

- Added a bounded linked-text projection for paragraph, heading, list, and table cell text in generated PDFs.
- Emitted active PDF `/Link` annotations with `/A << /S /URI /URI (...) >>` for safe external `http`, `https`, and `mailto` links.
- Placed link rectangles over approximate text-run areas derived from the existing lightweight line layout.
- Capped link annotation objects per exported document and per page. Over-budget links continue to render as visible text without annotation objects.
- Preserved page-range export behavior so selected pages include only their own in-budget annotations.
- Added focused tests for safe URI annotations, unsafe/internal link omission, page-range filtering, text-run rectangle bounds, per-page caps, and per-document caps.

## Implementation Notes

- PDF link safety reuses the existing hyperlink allowlist and applies a PDF-specific URI boundary: internal fragments are skipped, very large URI strings are skipped, and URI values with control characters or whitespace are skipped.
- Annotation rectangles use the same fixed-width estimate as the PDF wrapping path. This keeps clickable areas bounded to line text instead of page-wide areas, but it is not glyph-perfect.
- The PDF writer still emits deterministic generated objects without new PDF dependencies, network behavior, telemetry, cloud behavior, accounts, or remote resource loading.
- Unsafe link href values are not written to annotation objects. The PDF may only contain the visible linked label that was already rendered as document text.

## Compatibility Boundary

- ODT remains the canonical saved format. PDF export is still a conversion snapshot and does not update the current save path.
- Safe internal `#bookmark` links remain visible text only in generated PDFs.
- Header/footer links, comments, footnotes/endnotes, and table-of-contents entries do not gain active PDF annotation semantics in this sprint.

## Deferred

- Active internal PDF destinations for safe `#bookmark` links.
- Exact glyph-level link geometry and complex script shaping.
- PDF comment annotations and note backlinks.
- Full office-suite PDF layout compatibility.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-export`
- `cargo test --workspace`
- `./scripts/verify-public-release.sh`
- `git diff --check`
- `./scripts/verify-local.sh`

Reviewer follow-up added explicit coverage for annotation rectangle bounds and the 512-per-document annotation cap before the full local gate passed.
