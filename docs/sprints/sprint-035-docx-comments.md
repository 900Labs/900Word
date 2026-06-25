# Sprint 035: DOCX Comments MVP

## Scope

Sprint 035 extends the existing conversion-only `word-docx` boundary with simple DOCX comments while keeping ODT canonical and local-first privacy intact.

Included:

- Import simple legacy DOCX comments from safe local comments relationships into existing `word_core::CommentThread` values.
- Anchor supported `w:commentRangeStart` / `w:commentRangeEnd` ranges to visible inline text with `Inline.comment_ids`.
- Export valid 900Word-authored anchored comments as a generated `word/comments.xml` part, content type override, document relationship, range markers, and reference markers.
- Keep warnings generic and avoid exposing package entry names, relationship targets, local paths, private filenames, hostnames, usernames, or comment body text.

## Implementation Notes

- `word-docx` now recognizes the DOCX comments relationship type from `word/_rels/document.xml.rels`.
- Comments targets are accepted only when they resolve to simple `word/comments.xml` or `word/comments*.xml` parts that pass existing package preflight.
- Imported DOCX raw comment IDs are mapped to generated local `word-core` IDs. Unsafe or private target names are not preserved.
- Imported comment authors and bodies reuse `word-core` validation and normalization helpers.
- Comments are stored only after a supported visible text range proves the comment is anchored. Missing, malformed, point-only, duplicate, invalid, unanchored, over-limit, threaded/reply, or unsupported comments degrade with generic warnings.
- Export emits only comments that already exist in `word-core`, validate, and are anchored to supported visible inline text. Unanchored or invalid comments are not exported as hidden metadata.
- DOCX resolved-state fidelity is not emitted because the simple legacy comments representation has no matching bounded local mapping in this MVP.

## Verification

- `cargo test -p word-docx`
- `cargo test -p nine-hundred-word docx`
- `cargo fmt --all -- --check`
- `./scripts/verify-public-release.sh`
- `git diff --check`

## Deferred

- DOCX comment replies.
- Threaded comments and modern review extensions.
- DOCX resolved-state fidelity.
- Comments in unsupported ranges, headers, footers, notes, fields, drawings, or hidden metadata.
- DOCX tracked changes, notes, full review interoperability, and full DOCX fidelity.
