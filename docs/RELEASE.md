# Release Runbook

## Current Release Model

`v0.1.0` is a public source release. It publishes repository source, source archives, checksums, and release documentation for developers and early testers who build locally.

`v0.1.0` is not a signed binary installer release. Signed installers, notarization, package provenance, package privacy evidence, and automatic binary distribution are future hardening tasks.

Release-facing notes live in [v0.1.0 Release Notes Draft](releases/v0.1.0.md). Public-safe validation, CI, archive, checksum, SBOM, and compatibility placeholders live in [Release Evidence](RELEASE_EVIDENCE.md).

## Release Claim Freeze

Release-facing claims for `v0.1.0` are limited to:

- Local-first `.odt` document editing with ODT as the canonical saved format.
- Bounded DOCX import/export conversion for the supported subset, with degraded unsupported structures documented instead of hidden.
- Current export and print paths: TXT, sanitized HTML, lightweight PDF export, and sanitized WebView print preparation.
- No telemetry by default.

Do not describe broad Microsoft Word, LibreOffice, Google Docs, or ONLYOFFICE compatibility unless the claim is backed by generated or sanitized placeholder compatibility evidence from the release candidate commit.

## Pre-Tag Checklist

```bash
git checkout main
git pull --ff-only origin main
npm run verify:version
npm run verify:release-candidate
```

Confirm:

- `CHANGELOG.md` is updated.
- `docs/releases/v0.1.0.md` is updated without changing the source-only release scope.
- `docs/RELEASE_EVIDENCE.md` has public-safe pre-tag rows filled from release-candidate and CI evidence, with tag-generated release workflow rows still marked pending until the workflow completes.
- Sprint records are updated.
- Version surfaces match the root package version: package manifests, package lock entries, Tauri config, Cargo workspace package version, and the visible About/version string.
- `README.md`, `docs/PUBLIC_RELEASE.md`, `docs/releases/v0.1.0.md`, and this runbook describe `v0.1.0` as a public source release, not a binary installer release.
- Known limitations and deferred scope are documented for open-source users.
- ADRs are current.
- CI is green.
- The release-candidate gate runs from a clean worktree.
- Bundle-size budget passes.
- Performance smoke output is captured in release notes.
- Runtime offline source/capability scan passes.
- SBOM exists at `target/sbom/900word-sbom.json`.
- Public-release checklist passes.
- Git connectivity/integrity check passes.
- Compatibility evidence is recorded for any claimed external office-suite behavior.

For a binary package candidate, also confirm:

- `.github/workflows/package-test.yml` completed successfully for the candidate commit.
- Package-test run IDs are recorded.
- Package artifact scan passed for each platform bundle.
- Missing artifacts were treated as workflow failures.
- Manual runtime network monitoring evidence is attached before publishing binary artifacts.
- Compatibility testing uses generated or sanitized placeholder documents only and records application versions without local paths.

## Tag Flow

```bash
git tag v0.1.0
git push origin v0.1.0
```

The release workflow creates source archives, checksums, and an SBOM artifact after the tag is pushed.

After the workflow completes, copy only public-safe run IDs, artifact names, source archive checksum values, and SBOM artifact references into the GitHub release notes or a later public-safe evidence update. Do not copy local paths, hostnames, screenshots, private documents, or machine-specific notes.

## Binary Artifacts

Binary artifacts are out of scope for `v0.1.0`.

Use `.github/workflows/package-test.yml` for future manual package validation. Do not publish binary artifacts until package privacy scanning, manual runtime network evidence, signing policy, notarization or platform-equivalent signing, and package provenance are complete.
