# Release Evidence

This ledger is the public-safe evidence template for the `v0.1.0` source release. Keep placeholders until final release evidence is available from the release candidate commit, CI, and release workflow.

Pre-tag rows must be filled before tagging. Rows produced by the tag-triggered release workflow remain placeholders in the tagged source until that workflow completes; copy those values into the GitHub release notes or a later public-safe evidence update before publishing the final release page.

Do not record local computer paths, hostnames, personal names, private documents, screenshots, account identifiers, cloud URLs, private endpoints, generated package outputs, or machine-specific notes.

## v0.1.0 Source Release Evidence

| Evidence Item | Placeholder | Release Timing | Status |
| --- | --- | --- | --- |
| Final commit SHA | `<commit-sha>` | Yes, before tag | `<pending>` |
| Release PR | `<pull-request-url-or-number>` | Yes, before merge/tag | `<pending>` |
| Required CI run | `<ci-run-url-or-id>` | Yes, before tag | `<pending>` |
| Release workflow run | `<release-workflow-run-url-or-id>` | After tag or manual source-release run, before final release page publication | `<pending>` |
| Source archive artifact | `<source-archive-artifact-name>` | After release workflow, before final release page publication | `<pending>` |
| Source archive SHA-256 | `<sha256-from-SHA256SUMS.txt>` | After release workflow, before final release page publication | `<pending>` |
| SBOM artifact | `<sbom-artifact-name-or-run-id>` | After release workflow, before final release page publication | `<pending>` |
| Version consistency | `npm run verify:version` | Yes, before tag | `<pending>` |
| Frontend check | `npm run check` | Yes, before merge/tag | `<pending>` |
| Frontend lint | `npm run lint` | Yes, before merge/tag | `<pending>` |
| Frontend tests | `npm run test` | Yes, before merge/tag | `<pending>` |
| Public source scan | `./scripts/verify-public-release.sh` | Yes, before tag | `<pending>` |
| Release-candidate gate | `npm run verify:release-candidate` | Yes, from clean tag-ready checkout | `<pending>` |
| Bundle budget and performance smoke | `<release-candidate-validation-summary>` | Yes, before tag | `<pending>` |
| Compatibility evidence status | `<not-run-or-evidence-summary>` | Before any external-suite compatibility claim | `<pending>` |
| Binary package status | `out-of-scope-for-v0.1.0` | Must remain out of scope for this release | `<pending>` |

## Validation Command Log Template

Use this table for final release notes or a release issue. Record command names and public-safe pass/fail status only.

| Command | Commit | Result | Public-Safe Notes |
| --- | --- | --- | --- |
| `npm run verify:version` | `<commit-sha>` | `<pass-or-fail>` | `<notes>` |
| `npm run check` | `<commit-sha>` | `<pass-or-fail>` | `<notes>` |
| `npm run lint` | `<commit-sha>` | `<pass-or-fail>` | `<notes>` |
| `npm run test` | `<commit-sha>` | `<pass-or-fail>` | `<notes>` |
| `git diff --check` | `<commit-sha>` | `<pass-or-fail>` | `<notes>` |
| `./scripts/verify-public-release.sh` | `<commit-sha>` | `<pass-or-fail>` | `<notes>` |
| `npm run verify:release-candidate` | `<commit-sha>` | `<pass-or-fail>` | `<clean-checkout-required>` |

## Compatibility Evidence Status

Source-only `v0.1.0` may ship with external applications marked `not-run` if public wording avoids broad compatibility claims. Do not upgrade any row to `pass` or `degraded-documented` without generated or sanitized placeholder evidence from the release candidate commit.

| Application | Format Scope | Evidence Status | Evidence Reference | Notes |
| --- | --- | --- | --- | --- |
| LibreOffice Writer | `.odt`, `.docx`, `.pdf`, `.html` | `<not-run>` | `<artifact-or-run-id>` | `<public-safe-notes>` |
| Microsoft Word | `.docx`, `.pdf` | `<not-run>` | `<artifact-or-run-id>` | `<public-safe-notes>` |
| Google Docs | `.docx`, `.html`, `.pdf` | `<not-run>` | `<artifact-or-run-id>` | `<public-safe-notes>` |
| ONLYOFFICE Desktop Editors | `.docx`, `.odt`, `.pdf` | `<not-run>` | `<artifact-or-run-id>` | `<public-safe-notes>` |

See [Compatibility Testing](COMPATIBILITY_TESTING.md) for the fixture requirements, accepted result values, and privacy rules.

## Binary Evidence Boundary

Binary package evidence is intentionally deferred for `v0.1.0`. Do not add package-test run IDs, installer checksums, package screenshots, notarization records, signing identities, package provenance claims, or runtime network-monitoring evidence to this source-release ledger unless a later scoped release sprint changes the release model.
