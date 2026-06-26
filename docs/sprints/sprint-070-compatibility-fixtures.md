# Sprint 070: Compatibility Fixture Artifacts

## Goal

Provide generated placeholder artifacts for manual external office-suite compatibility testing without requiring private documents, real sample files, screenshots, logs, accounts, cloud behavior, telemetry, or committed binary artifacts.

## Changes

- Added `word_fixtures::compatibility_sample()` with generated placeholder content covering headings, TOC entries, formatted paragraphs, inline styling, lists, tables, image metadata, comments, tracked changes, footnotes, headers, footers, and page fields.
- Added a `word-fixtures` example generator that writes ODT, DOCX, TXT, HTML, print HTML, PDF, and a local README under an ignored output directory.
- Added fixture export smoke coverage for the supported generated formats.
- Updated compatibility, quality, public-release, roadmap, and changelog documentation with the generator command and privacy boundaries.

## Usage

```bash
cargo run -p word-fixtures --example generate_compatibility_artifacts -- target/compatibility
```

The generated artifacts must remain out of Git. Regenerate them from the release candidate commit being tested.

## Privacy And Security

- The fixture content is generated placeholder text only.
- The generated artifacts are written under ignored scratch output by default.
- The fixture avoids local paths, hostnames, usernames, personal names, account handles, email addresses, private filenames, and real document content.
- The generator does not fetch remote resources, automate external office suites, start cloud sync, or add telemetry.

## Out Of Scope

- Committing generated `.odt`, `.docx`, `.pdf`, `.html`, `.txt`, screenshots, logs, or suite-specific recovery files.
- Using real user, customer, school, NGO, legal, medical, financial, or personal documents.
- Claiming broad Microsoft Word, Google Docs, LibreOffice, or ONLYOFFICE compatibility without completed manual evidence.
- External office-suite automation, cloud import automation, accounts, telemetry, network behavior, and plugin/downloadable fixture stores.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p word-fixtures compatibility`
- `cargo test -p word-fixtures`
- `cargo run -p word-fixtures --example generate_compatibility_artifacts -- target/compatibility`
- `git diff --check`
- `./scripts/verify-public-release.sh`
