#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

fail() {
  echo "ERROR: $1" >&2
  exit 1
}

command -v rg >/dev/null 2>&1 || fail "rg is required"

echo "==> Checking frontend startup sources for browser network primitives"
if rg -n -I 'fetch\s*\(|XMLHttpRequest|WebSocket|EventSource|sendBeacon|navigator\.sendBeacon' \
  apps/desktop/src; then
  fail "runtime network primitive found in startup source"
fi

echo "==> Checking Rust backend for direct network clients"
if rg -n -I 'std::net::|tokio::net::|reqwest::|ureq::|hyper::Client|TcpStream|UdpSocket' \
  apps/desktop/src-tauri/src \
  crates; then
  fail "runtime backend network primitive found"
fi

echo "==> Checking Tauri default capability for broad network or shell access"
if rg -n -I '"shell|http:|https:|core:http|core:shell' apps/desktop/src-tauri/capabilities/default.json; then
  fail "default capability grants shell or network access"
fi

echo "==> Checking Tauri CSP stays local-only"
if ! rg -n -I "connect-src 'self' ipc: http://ipc.localhost" apps/desktop/src-tauri/tauri.conf.json >/dev/null; then
  fail "expected local-only Tauri connect-src was not found"
fi

echo "Runtime offline scan passed"
