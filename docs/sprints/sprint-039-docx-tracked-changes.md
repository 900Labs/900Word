# Sprint 039: DOCX Tracked Changes MVP

## Scope

Sprint 039 extends the existing conversion-only `word-docx` boundary with simple text-only DOCX tracked changes while keeping ODT canonical and local-first privacy intact.

Included:

- Import simple `w:ins` and `w:del` revision wrappers from body paragraphs, list items, and table cells when they contain supported visible text runs.
- Map imported revision text to existing `word_core::Inline.tracked_change` metadata with generated safe local change IDs.
- Export valid 900Word-authored text-only insertion/deletion changes as simple generated `w:ins` and `w:del` / `w:delText` markup.
- Keep warnings generic and avoid exposing package entry names, raw revision IDs, local paths, private filenames, hostnames, usernames, account metadata, or source filenames.

## Implementation Notes

- `word-docx` recognizes only `w:ins` and `w:del` as structured tracked-change containers.
- Imported DOCX revision IDs are not preserved. The converter generates local IDs such as `chg-docx-change-1`.
- Imported author metadata is bounded and sanitized. Obvious path-like, account-like, hostname-like, missing, or over-limit authors fall back to `External Reviewer`.
- Imported RFC3339 revision dates are preserved when safe; missing or malformed dates use a deterministic epoch fallback.
- Unsupported, nested, malformed, over-limit, move, formatting-only, property, table-row, field, media, or comment-as-change review markup degrades with generic warnings and visible text fallback where text is available.
- Export uses generated numeric DOCX revision IDs and sanitized local tracked-change author/date metadata. Unsafe local author strings fall back to `Local User`.
- If an inline has both comments and tracked-change metadata, DOCX export keeps the tracked-change markup and follows the existing conservative behavior of not exporting the comment anchor for that inline.
- External hyperlinks on tracked-change inlines are exported as revision text, not as active DOCX hyperlink relationships, because link-plus-review fidelity is deferred.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-docx`
- `cargo clippy -p word-docx -- -D warnings`

## Deferred

- Formatting-only tracked changes.
- Hyperlink-specific revision fidelity.
- Table row, table structure, image, page-region, field, and note changes.
- Move-from/move-to review fidelity.
- DOCX resolved-state fidelity, compare/merge, review filters, and full Word review interoperability.
- Comments-as-changes and combined comment/tracked-change round-trip fidelity.
- Full DOCX round-trip fidelity.

## Privacy Notes

- DOCX tracked-change import does not read OS usernames, hostnames, accounts, local paths, source filenames, contacts, cloud identity, or network state.
- Imported raw DOCX revision IDs and unsafe author metadata are not preserved.
- Exported DOCX revision IDs are generated, and exported author metadata is sanitized from local tracked-change metadata only.
- Tracked changes can reveal edit history and deleted text in shared files. Accept or reject pending changes before sharing when edit history is sensitive.
