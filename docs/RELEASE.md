# Release Runbook

## Current Release Model

The bootstrap release model publishes source artifacts only. Signed installers, notarization, package provenance, and automatic binary distribution are future hardening tasks.

## Pre-Tag Checklist

```bash
git checkout main
git pull --ff-only origin main
./scripts/verify-local.sh
./scripts/verify-public-release.sh
npm run sbom
```

Confirm:

- `CHANGELOG.md` is updated.
- Sprint records are updated.
- ADRs are current.
- CI is green.
- Bundle-size budget passes.
- Performance smoke output is captured in release notes.
- Runtime offline source/capability scan passes.
- SBOM exists at `target/sbom/900word-sbom.json`.
- Public-release checklist passes.

For a binary package candidate, also confirm:

- `.github/workflows/package-test.yml` completed successfully for the candidate commit.
- Package-test run IDs are recorded.
- Package artifact scan passed for each platform bundle.
- Missing artifacts were treated as workflow failures.
- Manual runtime network monitoring evidence is attached before publishing binary artifacts.

## Tag Flow

```bash
git tag v0.1.0
git push origin v0.1.0
```

The release workflow creates source archives, checksums, and an SBOM artifact.

## Binary Artifacts

Use `.github/workflows/package-test.yml` for manual package validation. Do not publish binary artifacts until package privacy scanning, manual runtime network evidence, and signing policy are complete.
