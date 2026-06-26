# Sprint 069: Compatibility Test Matrix

## Goal

Add a repeatable, privacy-safe manual compatibility evidence process before 900Word makes public claims about external office-suite behavior.

## Completed

- Added `docs/COMPATIBILITY_TESTING.md` with required generated/sanitized fixtures, target applications, result values, evidence template, and release gates.
- Linked compatibility testing from the documentation index.
- Added compatibility evidence requirements to the quality gate, public release checklist, and release runbook.
- Documented that unit/golden/package tests are not enough to claim Microsoft Word, Google Docs, LibreOffice, or ONLYOFFICE compatibility.

## Boundaries

- This sprint does not claim any external application is compatible.
- Compatibility artifacts, exported files, screenshots, app logs, suite recovery files, and generated packages remain outside Git.
- Generated or sanitized placeholder documents are required; real user documents are not allowed.
- Google Docs testing is allowed only as external compatibility evidence for generated placeholder documents and does not add cloud sync, account requirements, telemetry, or network behavior to 900Word.
- Automated browser/cloud upload tests, account-based workflows, broad marketing claims, and office-suite-specific workarounds remain deferred until separately scoped.

## Verification

- `git diff --check`
- changed-file privacy scan for local path and personal-info patterns
- `./scripts/verify-public-release.sh`

## Privacy Notes

- The matrix forbids real private documents and local path details in compatibility evidence.
- Evidence records use generic OS/application versions, commit SHAs, artifact/run IDs, and documented degradations.
- Any private metadata exposure, repair prompt, or undocumented content loss is release-blocking until fixed or documented.
