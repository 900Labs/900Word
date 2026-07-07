#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

fail() {
  echo "ERROR: $1" >&2
  exit 1
}

run() {
  echo
  echo "==> $*"
  "$@"
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "$1 is required"
}

require_clean_status() {
  local status
  status="$(git status --short)"
  if [[ -n "$status" ]]; then
    echo "$status" >&2
    fail "release candidates require a clean worktree"
  fi
}

require_clean_tracked_status() {
  local status
  status="$(git status --short --untracked-files=no)"
  if [[ -n "$status" ]]; then
    echo "$status" >&2
    fail "verification changed tracked files"
  fi
}

require_command git
require_command npm

run git rev-parse --verify HEAD
run git fsck --connectivity-only --no-dangling

echo
echo "==> Checking release candidate worktree status"
require_clean_status

run npm run verify:version
run ./scripts/verify-public-release.sh
run ./scripts/verify-local.sh

echo
echo "==> Checking tracked status after verification"
require_clean_tracked_status

echo
echo "Release candidate verification passed"
