# Compatibility Testing

900Word must not claim broad office-suite compatibility from unit tests alone. This matrix is the release evidence process for external applications.

## Scope

Run this matrix before making public claims such as "opens in Word", "works in Google Docs", or "compatible with ONLYOFFICE". Unit and golden tests remain required, but this manual matrix records how exported and round-tripped files behave in real applications.

## Privacy Rules

- Use generated 900Word documents or sanitized placeholder documents only.
- Do not use real user, customer, school, NGO, legal, medical, financial, or personal documents.
- Keep generated artifacts out of Git, such as under `target/compatibility/` or another ignored local scratch folder.
- Do not commit exported `.odt`, `.docx`, `.pdf`, `.html`, screenshots, app logs, or suite-specific recovery files.
- Do not record local paths, usernames, hostnames, account names, cloud URLs, private filenames, personal handles, or machine identifiers in notes.
- Google Docs testing must use a generated placeholder document only and must not be used as evidence for any cloud feature in 900Word.

## Required Fixture Set

Create the fixtures from generated or placeholder content:

| Fixture | Required Content | Native Source |
| --- | --- | --- |
| Formal letter | title, paragraphs, date, footer/page number | generated template or placeholder document |
| School report | H1-H3 headings, TOC, page numbers, image with alt/caption, table, footnote | generated placeholder content |
| CV/resume | styles, lists, links, section headings | generated template |
| NGO/project report | table, image, comments, tracked changes, header/footer | generated placeholder content |
| Review document | comments, tracked insertions/deletions, footnote/endnote | generated placeholder content |
| DOCX received document | simple `.docx` imported into 900Word and exported back | generated DOCX fixture only |

## Applications

Record exact application versions. Do not mark an application as passing without opening the file and checking visible content.

| Application | Formats | Required Result |
| --- | --- | --- |
| LibreOffice Writer | `.odt`, `.docx`, `.pdf`, `.html` | opens without repair prompt; supported content visible; documented degradations match file-format docs |
| Microsoft Word | `.docx`, `.pdf` | opens without repair prompt; supported content visible; no private metadata surfaced |
| Google Docs | `.docx`, `.html`, `.pdf` | imports generated placeholder files acceptably; no 900Word cloud or sync claim is made |
| ONLYOFFICE Desktop Editors | `.docx`, `.odt`, `.pdf` | opens without repair prompt; supported content visible; documented degradations match file-format docs |

## Result Values

- `pass`: supported content appears as expected and no repair/privacy issue is observed.
- `degraded-documented`: unsupported behavior is visible but already documented in `docs/FILE_FORMATS.md`.
- `fail`: content loss, corruption, repair prompt, private metadata exposure, or undocumented degradation.
- `not-run`: no evidence yet.

## Evidence Template

Use this template in release notes or a release issue. Keep paths generic.

```md
## Compatibility Evidence

Commit: <commit-sha>
900Word package/test artifact: <artifact-name-or-run-id>
Tester: <non-identifying role or reviewer label>
Date: YYYY-MM-DD
Operating system: <generic OS/version>

| Fixture | Format | Application/version | Result | Notes |
| --- | --- | --- | --- | --- |
| School report | `.docx` | LibreOffice Writer <version> | pass | Supported headings, TOC text, table, image, note visible. |
| School report | `.docx` | Microsoft Word <version> | not-run | Not yet tested. |
```

## Release Gates

- Source-only releases may ship with `not-run` external applications if public wording avoids broad compatibility claims.
- Binary releases should include at least one local office-suite pass for `.odt`, `.docx`, and `.pdf`.
- Do not claim Microsoft Word, Google Docs, LibreOffice, or ONLYOFFICE compatibility unless the matching matrix row is `pass` or `degraded-documented` for the release candidate.
- Any `fail` result must either block the release or be documented as a known limitation before release notes are published.
