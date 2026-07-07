# Sprint 076: v0.1.0 Source Release

## Scope

Prepare the final docs/process patch required before tagging `v0.1.0` as a public source release. This sprint is docs-only and does not change app code, workflow code, package scripts, generated fixtures, generated artifacts, telemetry, network behavior, cloud sync, accounts, collaboration, plugin runtime behavior, binary packaging, signing, notarization, automatic updates, or full encryption scope.

## Deliverables

- Dated `CHANGELOG.md` for `0.1.0 - 2026-07-07`.
- Finalized `docs/releases/v0.1.0.md` as source-release notes suitable for GitHub release use.
- Corrected `docs/RELEASE_EVIDENCE.md` so tagged source files contain validation boundaries and external evidence templates instead of impossible self-referential commit SHA requirements.
- Updated release runbook, public release checklist, documentation links, and roadmap language to separate in-source validation instructions from GitHub release notes or post-release evidence updates.

## Validation

- `git diff --check`
- `npm run verify:version`
- `./scripts/verify-public-release.sh`
- Scoped privacy grep for local paths, personal names, hostnames, private endpoints, screenshots, and binary-release overclaims in the changed release docs.

## Decisions

- `v0.1.0` remains a public source release for developers and early testers who build locally.
- Concrete tag target, commit SHA, CI run, release workflow run, source archive checksum, and SBOM evidence must be recorded in GitHub release notes or a later public-safe evidence update after those values exist.
- Tagged source files may contain validation instructions, public-safe templates, and conservative source-release boundaries, but not a filled value that can only be known after the containing commit exists.
- Signed installers, notarization, package provenance, automatic updates, package privacy evidence, binary artifacts, broad office-suite compatibility claims, cloud sync, collaboration, plugin runtime, telemetry, accounts, and full encryption remain out of scope.

## Follow-ups

- After this sprint merges, run `npm run verify:release-candidate` from a clean tag-ready checkout.
- Tag `v0.1.0`, push the tag, and wait for the release workflow to produce source archives, checksums, and SBOM evidence.
- Publish GitHub release notes with public-safe commit/tag, CI, release workflow, source archive checksum, SBOM, and compatibility status evidence.
