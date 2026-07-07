# Sprint 074: Release Candidate Hardening

## Scope

Harden the `v0.1.0` public source release candidate process without adding UI features, changing export backends, changing package artifact publication policy, or expanding telemetry, network, cloud, account, or plugin behavior.

## Deliverables

- Added `scripts/verify-version-consistency.mjs` to compare the root package version against package-lock release entries, the desktop workspace package, Tauri config, Cargo workspace package metadata, and the visible About/version string.
- Added `npm run verify:version` as a cheap release-facing consistency check.
- Added `scripts/verify-release-candidate.sh` and `npm run verify:release-candidate` for clean tag-ready checkouts and source-release CI.
- Updated `scripts/verify-local.sh` so local verification fails early when version surfaces drift.
- Updated the source-release workflow to install Cargo security tools when needed and run the shared release-candidate gate instead of a weaker duplicated subset.
- Updated release, public-release, roadmap, and changelog documentation with the new maintainer process.

## Validation

- `npm run verify:version`
- `bash -n scripts/verify-release-candidate.sh scripts/verify-local.sh`
- `node --check scripts/verify-version-consistency.mjs`
- `npm run verify:release-candidate` dirty-checkout guard exercised against the uncommitted sprint branch; run the full clean gate after committing and before tagging.
- `git diff --check`
- `./scripts/verify-public-release.sh`

## Decisions

- The version checker derives the expected version from root `package.json` so future releases update one canonical source before consistency verification.
- The release-candidate gate requires a clean worktree because it is intended for pre-tag and CI validation, not ordinary feature-branch iteration.
- `verify-local.sh` includes version consistency, but clean-status enforcement stays in the release-candidate wrapper.
- The source release remains source-only. Signed installers, notarization, automatic updates, package provenance, and binary artifact publication remain deferred.

## Follow-ups

- Run the release-candidate gate on the final `v0.1.0` commit before tagging.
- Keep release notes tied to generated or sanitized placeholder compatibility evidence for any external office-suite claims.
- Keep binary package validation, signing, notarization, package provenance, and runtime network monitoring evidence out of `v0.1.0` unless a future scoped release sprint explicitly adds them.
