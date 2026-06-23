# Sprint 008: Release Hardening

## Status

Complete.

## Delivered

- Added bundle-size budget checks for desktop frontend build output.
- Added package artifact scanner for built bundle directories, with local-path, hostname, private-key, debug-file, and development-server URL checks.
- Added runtime offline source/capability scan for startup network primitives, broad network permissions, shell permissions, and Tauri local-only CSP.
- Added SBOM generation from `Cargo.lock`/Cargo metadata and `package-lock.json`.
- Added performance smoke script that records desktop build-output size and smoke timings for export and ODT round-trip tests.
- Wired Sprint 008 gates into `./scripts/verify-local.sh`, CI, release workflow, and package-test workflow.
- Hardened package-test workflow so missing artifacts fail and package outputs are scanned before upload.
- Updated release, quality, public-release, performance-budget, and ADR documentation.

## Validation

- `./scripts/verify-local.sh`
- Desktop frontend build output: `259880` bytes in local smoke.
- `npm run smoke:offline`
- `npm run scan:packages`
- `npm run sbom`
- `npm run smoke:performance`
- Remote CI pending after Sprint 008 commit.

## Follow-Ups

- Add packet-level runtime network monitoring evidence before publishing binary artifacts.
- Add signed/notarized package provenance before binary distribution.
- Replace smoke timings with platform-specific startup, idle memory, typing-latency, ODT open/save, PDF export, and installer-size baselines.
