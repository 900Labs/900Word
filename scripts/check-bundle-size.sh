#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

DIST_DIR="${DESKTOP_DIST_DIR:-apps/desktop/dist}"
MAX_DESKTOP_DIST_BYTES="${MAX_DESKTOP_DIST_BYTES:-2500000}"
MAX_DESKTOP_ASSET_BYTES="${MAX_DESKTOP_ASSET_BYTES:-750000}"

if [[ ! -d "$DIST_DIR" ]]; then
  echo "ERROR: desktop build output is missing at $DIST_DIR; run npm run build first" >&2
  exit 1
fi

total_bytes=0
largest_file=""
largest_bytes=0

while IFS= read -r -d '' file; do
  bytes="$(wc -c < "$file" | tr -d '[:space:]')"
  total_bytes=$((total_bytes + bytes))
  if (( bytes > largest_bytes )); then
    largest_bytes="$bytes"
    largest_file="$file"
  fi
  if (( bytes > MAX_DESKTOP_ASSET_BYTES )); then
    echo "ERROR: bundle asset exceeds ${MAX_DESKTOP_ASSET_BYTES} bytes: $file ($bytes bytes)" >&2
    exit 1
  fi
done < <(find "$DIST_DIR" -type f -print0)

echo "Desktop dist bytes: $total_bytes"
echo "Largest desktop asset bytes: $largest_bytes"

if (( total_bytes > MAX_DESKTOP_DIST_BYTES )); then
  echo "ERROR: desktop dist exceeds ${MAX_DESKTOP_DIST_BYTES} bytes" >&2
  echo "Largest file: $largest_file" >&2
  exit 1
fi

echo "Bundle size budget passed"
