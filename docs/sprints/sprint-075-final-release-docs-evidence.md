# Sprint 075: Final Release Docs Evidence

## Scope

Prepare final public source release documentation and evidence surfaces for tagging `v0.1.0` soon. This sprint is docs-only and does not change app features, package scripts, package workflows, generated fixtures, package artifacts, telemetry, network behavior, cloud sync, accounts, or plugin runtime behavior.

## Deliverables

- Added `docs/releases/v0.1.0.md` as the unreleased public source release notes draft.
- Added `docs/RELEASE_EVIDENCE.md` as a placeholder-only evidence ledger for release PR, required CI run, release workflow run, source archive checksum, SBOM artifact, validation commands, compatibility evidence status, and the final commit SHA evidence boundary.
- Updated the root README, documentation index, release runbook, public release checklist, changelog, and roadmap so the release notes and evidence ledger are discoverable.
- Kept `CHANGELOG.md` at `0.1.0 - Unreleased` so a later release sprint can date and tag it.

## Validation

- `npm run verify:version`
- `npm run check`
- `npm run lint`
- `npm run test`
- `git diff --check`
- `./scripts/verify-public-release.sh`

## Decisions

- `v0.1.0` remains a public source release for developers and early testers who build locally.
- Signed installers, notarization, package provenance, automatic updates, package privacy evidence, and binary artifact publication remain deferred.
- Release evidence uses placeholders until final values come from the release candidate commit, CI, and release workflow; tag-generated artifact rows remain pending until the release workflow completes.
- Compatibility status remains `not-run` unless generated or sanitized placeholder evidence supports a narrower claim.

## Follow-ups

- Record public-safe release-candidate, CI, release workflow, source archive checksum, and SBOM evidence in GitHub release notes or a later evidence update before publishing the final release page.
- Run `npm run verify:release-candidate` from a clean tag-ready checkout before tagging `v0.1.0`.
- Keep binary packaging, signing, notarization, package provenance, and runtime network monitoring evidence scoped to a future binary release.
