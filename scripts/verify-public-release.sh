#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

fail() {
  echo "ERROR: $1" >&2
  exit 1
}

command -v rg >/dev/null 2>&1 || fail "rg is required"

echo "==> Checking for OS metadata files"
if find . \( -path './.git' -o -path './node_modules' -o -path './target' -o -path './apps/desktop/dist' -o -path './apps/desktop/src-tauri/target' \) -prune -o \
  \( -name .DS_Store -o -name Thumbs.db -o -name desktop.ini \) -print | grep -q .; then
  find . \( -path './.git' -o -path './node_modules' -o -path './target' -o -path './apps/desktop/dist' -o -path './apps/desktop/src-tauri/target' \) -prune -o \
    \( -name .DS_Store -o -name Thumbs.db -o -name desktop.ini \) -print >&2
  fail "OS metadata files are present"
fi

echo "==> Checking for local paths"
if rg -n -I '(/Users/[A-Za-z0-9._-]+|/home/[A-Za-z0-9._-]+|C:\\Users\\[^\\]+|Desktop/[A-Za-z0-9._-]+)' . \
  --glob '!scripts/verify-public-release.sh' \
  --glob '!package-lock.json' \
  --glob '!target/**' \
  --glob '!apps/desktop/dist/**' \
  --glob '!apps/desktop/src-tauri/target/**' \
  --glob '!node_modules/**'; then
  fail "local path pattern found"
fi

echo "==> Checking for private key material"
if rg -n -I --fixed-strings \
  -e '-----BEGIN RSA PRIVATE KEY-----' \
  -e '-----BEGIN OPENSSH PRIVATE KEY-----' \
  -e '-----BEGIN PRIVATE KEY-----' \
  . \
  --glob '!scripts/verify-public-release.sh' \
  --glob '!target/**' \
  --glob '!apps/desktop/dist/**' \
  --glob '!apps/desktop/src-tauri/target/**' \
  --glob '!node_modules/**'; then
  fail "private key material found"
fi

echo "==> Checking for generated package artifacts"
if find . \( -path './.git' -o -path './node_modules' -o -path './target' -o -path './apps/desktop/dist' -o -path './apps/desktop/src-tauri/target' \) -prune -o \
  \( -name '*.dmg' -o -name '*.msi' -o -name '*.exe' -o -name '*.AppImage' -o -name '*.deb' -o -name '*.rpm' -o -name '*.app' -o -name '*.zip' \) -print | grep -q .; then
  fail "generated package artifact found"
fi

echo "==> Checking hostname leakage"
HOSTNAME_SHORT="$(hostname -s 2>/dev/null || true)"
if [[ -n "$HOSTNAME_SHORT" ]]; then
  if rg -n -I --fixed-strings "$HOSTNAME_SHORT" . \
    --glob '!scripts/verify-public-release.sh' \
    --glob '!package-lock.json' \
    --glob '!target/**' \
    --glob '!apps/desktop/dist/**' \
    --glob '!apps/desktop/src-tauri/target/**' \
    --glob '!node_modules/**'; then
    fail "local hostname found"
  fi
fi

echo "Public release source scan passed"
