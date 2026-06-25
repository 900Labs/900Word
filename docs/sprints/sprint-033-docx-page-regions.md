# Sprint 033: DOCX Page Regions

## Status

Complete.

## Scope

Sprint 033 improves the conversion-only DOCX boundary for simple page regions:

- Import default and first-page DOCX header/footer references into the existing `word-core` `PageRegions` model.
- Read only safe relationship-resolved `word/header*.xml` and `word/footer*.xml` package parts that already pass DOCX package preflight.
- Import simple paragraph/run content, supported inline marks, and recognized simple `PAGE`, `NUMPAGES`, and `DATE` fields.
- Export 900Word-authored default and first-page page regions as minimal DOCX header/footer parts, document relationships, and section references.
- Keep ODT as the canonical saved format.

## Implementation Notes

- DOCX document relationships now track safe local header and footer relationship types in addition to hyperlinks.
- Header/footer targets are rejected unless they resolve to simple package paths under `word/` for `header*.xml` or `footer*.xml`.
- Unsupported, missing, unsafe, or remote header/footer relationships degrade with generic warnings.
- First-page header/footer references set `different_first_page` in `word-core`.
- Even-page regions are warned and ignored for now because the current model and UI do not expose editable even-page variants.
- DOCX export emits only the page-region parts needed by the first section's `PageRegions` content.

## Verification

- Added a synthetic DOCX import test for header/footer text and page fields.
- Added a DOCX export/import round-trip test for 900Word-authored page regions.
- Added a hostile relationship target test to prove unsafe header/footer targets are ignored with generic warning behavior.

## Deferred

- DOCX images/media in page regions.
- DOCX comments, tracked changes, footnotes, and endnotes.
- Even-page headers/footers.
- Complex field fidelity beyond page number, page count, and date.
- Complex section layout and multi-section page-region fidelity.
- Full layout fidelity or deterministic DOCX pagination.
- Cloud, telemetry, accounts, network behavior, external converters, and heavyweight dependencies.
