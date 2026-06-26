# Public Release Checklist

900Word stays private until this checklist passes on `main`.

## Source Scan

```bash
./scripts/verify-public-release.sh
git status --short --branch
```

Expected result:

1. No local user paths.
2. No hostnames.
3. No secrets or private keys.
4. No OS metadata files.
5. No real user documents.
6. No generated package artifacts.
7. Runtime offline source/capability scan passes.
8. SBOM is generated under `target/sbom/900word-sbom.json`.
9. Bundle budget and performance smoke pass.

## Package Scan

Before publishing binary artifacts:

1. Build package artifacts with `.github/workflows/package-test.yml`.
2. Require package-test run IDs in release notes.
3. Ensure missing artifacts fail the workflow.
4. Run `node scripts/scan-package-artifacts.mjs <UNPACKED_OR_BUNDLE_DIR>` against each bundle directory.
5. Search unpacked files for local paths, hostnames, secrets, debug files, and private sample content.
6. Confirm no telemetry or unsolicited network calls during runtime smoke. Sprint 008 automates source/capability checks; packet-level runtime monitoring remains a manual release evidence item.

## Compatibility Evidence

Before making public compatibility claims:

1. Run the [Compatibility Testing](COMPATIBILITY_TESTING.md) matrix for the claimed application and format.
2. Use generated or sanitized placeholder documents only.
3. Record application versions, generic OS version, commit SHA, artifact/run ID, result, and documented degradations.
4. Keep exported files, screenshots, logs, and suite-specific recovery files out of Git.
5. Treat any undocumented content loss, repair prompt, or private metadata exposure as release-blocking until fixed or documented as a known limitation.

## Documentation Scan

Docs must use placeholders such as:

- `YOUR_GITHUB_USERNAME`
- `{REPO_ROOT}`
- `{APP_DATA_DIR}`

Do not include local computer paths or personal names.
