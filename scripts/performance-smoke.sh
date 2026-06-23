#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

measure() {
  local label="$1"
  shift
  local start
  local end
  start="$(date +%s)"
  "$@"
  end="$(date +%s)"
  echo "${label} seconds: $((end - start))"
}

if [[ ! -d apps/desktop/dist ]]; then
  echo "ERROR: desktop build output is missing; run npm run build first" >&2
  exit 1
fi

dist_bytes=0
while IFS= read -r -d '' file; do
  bytes="$(wc -c < "$file" | tr -d '[:space:]')"
  dist_bytes=$((dist_bytes + bytes))
done < <(find apps/desktop/dist -type f -print0)

echo "Desktop dist bytes: $dist_bytes"
measure "word-export smoke" cargo test -p word-export --quiet
measure "word-odf roundtrip smoke" cargo test -p word-odf generated_odt_round_trips_mvp_blocks_and_multilingual_text --quiet
echo "Performance smoke passed"
