#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

run() {
  echo
  echo "==> $*"
  "$@"
}

if [[ ! -d node_modules ]]; then
  run npm install
fi

run npm run check
run npm run lint
run npm run test
run cargo fmt --all -- --check
run cargo check --workspace
run cargo test --workspace
run cargo clippy --workspace -- -D warnings
run ./scripts/verify-public-release.sh

if command -v cargo-audit >/dev/null 2>&1; then
  run cargo audit
else
  echo
  echo "==> cargo-audit not installed; skipping local optional audit"
fi

if command -v cargo-deny >/dev/null 2>&1; then
  run cargo deny check
else
  echo
  echo "==> cargo-deny not installed; skipping local optional license check"
fi

run npm audit --audit-level=high

echo
echo "Local verification passed"
