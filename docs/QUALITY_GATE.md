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
- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `./scripts/verify-public-release.sh`

`cargo test --workspace` includes bootstrap security smoke tests for disabled telemetry, no frontend startup network primitives, and no default shell capability.

Optional tools:

- `cargo audit`
- `cargo deny check`
- `npm audit --audit-level=high`

If optional tools are unavailable locally, CI should install and run them.
