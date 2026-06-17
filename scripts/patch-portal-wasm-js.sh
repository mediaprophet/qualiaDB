#!/usr/bin/env bash
# Align qualia.js default wasm URL with docs/pkg/qualia/qualia_bg.wasm (Pages publish name).
set -euo pipefail
JS="${1:-docs/pkg/qualia/qualia.js}"
if [[ ! -f "$JS" ]]; then
  echo "patch-portal-wasm-js: missing $JS" >&2
  exit 1
fi
if grep -q "qualia_core_db_bg.wasm" "$JS"; then
  sed -i 's/qualia_core_db_bg\.wasm/qualia_bg.wasm/g' "$JS"
  echo "patch-portal-wasm-js: patched $JS → qualia_bg.wasm"
else
  echo "patch-portal-wasm-js: $JS already references qualia_bg.wasm"
fi