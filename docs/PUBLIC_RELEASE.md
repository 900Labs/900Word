# Public Release Checklist

900Word stays private until this checklist passes on `main`.

## v0.1.0 Release Scope

`v0.1.0` is a public source release for developers and early testers who build locally. It is not a signed binary installer release.

Public claims for `v0.1.0` are limited to:

1. Local-first `.odt` editing with ODT as the canonical saved format.
2. Bounded DOCX import/export conversion for the documented supported subset.
3. Current export and print paths: TXT, sanitized HTML, lightweight PDF export, and sanitized WebView print preparation.
4. No telemetry by default.

Do not claim broad Microsoft Word, LibreOffice, Google Docs, or ONLYOFFICE compatibility without release-candidate evidence from generated or sanitized placeholder documents.

## Known Limitations

Open-source users should expect these limitations in `v0.1.0`:

1. No official signed installers, notarized packages, automatic updates, or binary package support.
2. No cloud sync, accounts, collaboration, plugin runtime, remote document storage, macros, or legacy binary `.doc` support.
3. DOCX remains conversion-only and may degrade unsupported WordprocessingML structures with warnings.
4. Print and PDF output use the current lightweight export path and do not claim full print-layout or deterministic pagination fidelity.
5. External office-suite behavior is not a compatibility promise unless backed by the compatibility evidence process.
6. Full Word/LibreOffice parity, richer style management, full encryption, broad dictionary compatibility, and platform accessibility polish remain deferred.

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

## Deferred Binary Package Scan

Binary artifacts are out of scope for `v0.1.0`. Before any future binary artifact publication:

1. Build package artifacts with `.github/workflows/package-test.yml`.
2. Require package-test run IDs in release notes.
3. Ensure missing artifacts fail the workflow.
4. Run `node scripts/scan-package-artifacts.mjs <UNPACKED_OR_BUNDLE_DIR>` against each bundle directory.
5. Search unpacked files for local paths, hostnames, secrets, debug files, and private sample content.
6. Confirm no telemetry or unsolicited network calls during runtime smoke. Sprint 008 automates source/capability checks; packet-level runtime monitoring remains a manual release evidence item.

## Compatibility Evidence

Before making public compatibility claims:

1. Run the [Compatibility Testing](COMPATIBILITY_TESTING.md) matrix for the claimed application and format.
2. Generate the broad placeholder sample with `cargo run -p word-fixtures --example generate_compatibility_artifacts -- target/compatibility` from the release candidate commit.
3. Use generated or sanitized placeholder documents only.
4. Record application versions, generic OS version, commit SHA, artifact/run ID, result, and documented degradations.
5. Keep exported files, screenshots, logs, and suite-specific recovery files out of Git.
6. Treat any undocumented content loss, repair prompt, or private metadata exposure as release-blocking until fixed or documented as a known limitation.

## Documentation Scan

Docs must use placeholders such as:

- `YOUR_GITHUB_USERNAME`
- `{REPO_ROOT}`
- `{APP_DATA_DIR}`

Do not include local computer paths or personal names.
