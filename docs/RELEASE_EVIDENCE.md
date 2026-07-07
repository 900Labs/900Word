# Release Evidence

This document defines the public-safe evidence flow for the `v0.1.0` source release.

A tagged source file cannot contain the exact commit SHA of the commit that contains that file. Do not try to fill concrete release identifiers inside source files that are included in the same tag. The tagged source should contain validation instructions, release boundaries, and public-safe templates only.

Concrete release evidence belongs in GitHub release notes or a later public-safe evidence update after the value exists.

Do not record local computer paths, hostnames, personal names, private documents, screenshots, account identifiers, cloud URLs, private endpoints, generated package outputs, or machine-specific notes.

## In-Source Validation Boundary

These rows describe what must be true for the `v0.1.0` source release. They are intentionally not filled with concrete run IDs or commit SHAs in the tagged source.

| Gate | Required Boundary | Concrete Evidence Location |
| --- | --- | --- |
| Tag target commit SHA | Must identify the merged release commit before creating `v0.1.0` | GitHub release notes or post-release evidence update |
| Release PR | Must be merged with required CI passing before tag | GitHub release notes or post-release evidence update |
| Required CI run | Must pass for the release PR or final release commit before tag | GitHub release notes or post-release evidence update |
| Version consistency | `npm run verify:version` must pass before tag | Release PR, GitHub release notes, or post-release evidence update |
| Frontend check | `npm run check` must pass before tag | Release PR, GitHub release notes, or post-release evidence update |
| Frontend lint | `npm run lint` must pass before tag | Release PR, GitHub release notes, or post-release evidence update |
| Frontend tests | `npm run test` must pass before tag | Release PR, GitHub release notes, or post-release evidence update |
| Public source scan | `./scripts/verify-public-release.sh` must pass before tag | Release PR, GitHub release notes, or post-release evidence update |
| Release-candidate gate | `npm run verify:release-candidate` must pass from a clean tag-ready checkout before tag | GitHub release notes or post-release evidence update |
| Release workflow run | Must complete after tag push before publishing the final release page | GitHub release notes or post-release evidence update |
| Source archive artifact | Must be produced by the release workflow after tag push | GitHub release notes or post-release evidence update |
| Source archive SHA-256 | Must come from `SHA256SUMS.txt` after release workflow completion | GitHub release notes or post-release evidence update |
| SBOM artifact | Must be produced by the release workflow after tag push | GitHub release notes or post-release evidence update |
| Compatibility evidence status | Must remain `not-run` unless generated or sanitized placeholder evidence supports a narrower claim | GitHub release notes or post-release evidence update |
| Binary package status | Must remain `out-of-scope-for-v0.1.0` | GitHub release notes or post-release evidence update |

## External Evidence Template

Use this table in GitHub release notes, a release issue, or a later evidence update. Do not fill it inside the tagged source commit that it describes. Record command names and public-safe pass/fail status only.

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

Source-only `v0.1.0` may ship with external applications marked `not-run` if public wording avoids broad compatibility claims. Do not upgrade any row to `pass` or `degraded-documented` in GitHub release notes or a post-release evidence update without generated or sanitized placeholder evidence from the release candidate commit.

| Application | Format Scope | Evidence Status | Evidence Reference | Notes |
| --- | --- | --- | --- | --- |
| LibreOffice Writer | `.odt`, `.docx`, `.pdf`, `.html` | `<not-run>` | `<artifact-or-run-id>` | `<public-safe-notes>` |
| Microsoft Word | `.docx`, `.pdf` | `<not-run>` | `<artifact-or-run-id>` | `<public-safe-notes>` |
| Google Docs | `.docx`, `.html`, `.pdf` | `<not-run>` | `<artifact-or-run-id>` | `<public-safe-notes>` |
| ONLYOFFICE Desktop Editors | `.docx`, `.odt`, `.pdf` | `<not-run>` | `<artifact-or-run-id>` | `<public-safe-notes>` |

See [Compatibility Testing](COMPATIBILITY_TESTING.md) for the fixture requirements, accepted result values, and privacy rules.

## Binary Evidence Boundary

Binary package evidence is intentionally deferred for `v0.1.0`. Do not add package-test run IDs, installer checksums, package screenshots, notarization records, signing identities, package provenance claims, or runtime network-monitoring evidence to this source-release ledger unless a later scoped release sprint changes the release model.
