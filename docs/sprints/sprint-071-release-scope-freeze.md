# Sprint 071: Release Scope Freeze

## Scope

Freeze the `v0.1.0` public release story as a source release for developers and early testers. This sprint updates documentation only and does not change application code, build behavior, package workflows, fixtures, or generated artifacts.

## Deliverables

- Defined `v0.1.0` as a public source release, not a signed binary installer release.
- Limited release-facing claims to local ODT editing, bounded DOCX import/export conversion, current TXT/sanitized HTML/lightweight PDF export and sanitized print paths, and no telemetry by default.
- Added open-source-facing known limitations for binary packaging, DOCX conversion, external office-suite compatibility, cloud/collaboration, plugin runtime, deterministic pagination, full layout fidelity, full Word/LibreOffice parity, and other deferred scope.
- Updated the changelog and roadmap with a human-readable release-prep record.
- Kept the docs placeholder-safe with no personal names, local computer paths, hostnames, private documents, generated artifacts, or private data.

## Validation

- `./scripts/verify-public-release.sh`
- `git diff --check`

## Decisions

- `v0.1.0` should be described as a public source release for people building locally.
- Binary installers, notarization or platform-equivalent signing, package provenance, automatic updates, and package privacy evidence remain future release-hardening work.
- Compatibility language stays conservative: Microsoft Word, LibreOffice, Google Docs, and ONLYOFFICE behavior requires generated or sanitized placeholder evidence before public claims expand.
- DOCX remains a bounded conversion boundary; ODT remains the canonical saved format.

## Follow-ups

- Capture release-candidate validation evidence after the docs freeze lands.
- Generate a release-candidate SBOM and record the artifact location in release notes.
- Keep compatibility evidence outside Git and use generated or sanitized placeholder documents only.
- Scope binary packaging and signing policy separately before any installer release.
