# Release Runbook

## Current Release Model

The bootstrap release model publishes source artifacts only. Signed installers, notarization, package provenance, and automatic binary distribution are future hardening tasks.

## Pre-Tag Checklist

```bash
git checkout main
git pull --ff-only origin main
./scripts/verify-local.sh
./scripts/verify-public-release.sh
```

Confirm:

- `CHANGELOG.md` is updated.
- Sprint records are updated.
- ADRs are current.
- CI is green.
- Public-release checklist passes.

## Tag Flow

```bash
git tag v0.1.0
git push origin v0.1.0
```

The release workflow creates source archives and checksums.

## Binary Artifacts

Use `.github/workflows/package-test.yml` for manual package validation. Do not publish binary artifacts until package privacy scanning and signing policy are complete.
