# Quality Gate

Run from repository root before opening a pull request:

```bash
./scripts/verify-local.sh
```

The gate runs:

- `npm install` when dependencies are missing.
- `npm run check`
- `npm run lint`
- `npm run test`
- `npm run build`
- `npm run budget:bundle`
- `npm run smoke:offline`
- `npm run scan:packages`
- `npm run sbom`
- `npm run smoke:performance`
- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `./scripts/verify-public-release.sh`

`cargo test --workspace` includes bootstrap security smoke tests for disabled telemetry, no frontend startup network primitives, and no default shell capability. `npm run smoke:offline` repeats the source and capability scan as a release hardening gate. It is not a packet-level runtime monitor.

Sprint 008 hardening scripts:

- `scripts/check-bundle-size.sh` enforces the initial desktop build-output budget.
- `scripts/scan-package-artifacts.mjs` scans built package directories when present.
- `scripts/generate-sbom.mjs` writes `target/sbom/900word-sbom.json`.
- `scripts/performance-smoke.sh` records build-output size and export/ODT smoke timings.

Optional tools:

- `cargo audit`
- `cargo deny check`
- `npm audit --audit-level=high`

If optional tools are unavailable locally, CI should install and run them.

Manual compatibility evidence:

- Use [Compatibility Testing](COMPATIBILITY_TESTING.md) before making public claims about Microsoft Word, Google Docs, LibreOffice, or ONLYOFFICE behavior.
- Generate the broad placeholder sample with `cargo run -p word-fixtures --example generate_compatibility_artifacts -- target/compatibility` when preparing manual compatibility evidence.
- Unit, golden, and package tests are required but are not enough to claim external office-suite compatibility.
- Keep compatibility artifacts out of Git and use generated or sanitized placeholder documents only.
