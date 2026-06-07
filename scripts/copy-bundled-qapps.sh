#!/usr/bin/env bash
# Copy bundled qapps (Anatomy + WASM) into a desktop dist folder.
# Usage: ./scripts/copy-bundled-qapps.sh dist/qualia-flutter-linux-x64

set -euo pipefail

OUT_DIR="${1:?usage: copy-bundled-qapps.sh <out-dir>}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ANATOMY_SRC="$ROOT/bundled/qapps/Anatomy"
if [[ ! -d "$ANATOMY_SRC" ]]; then
  ANATOMY_SRC="$ROOT/app-development/Anatomy"
fi
WASM_SRC="$ROOT/docs/playground"
DEST="$OUT_DIR/bundled/qapps/Anatomy"

if [[ ! -d "$ANATOMY_SRC" ]]; then
  echo "warning: Anatomy source not found at $ANATOMY_SRC — skipping" >&2
  exit 0
fi

echo "Copying Anatomy qapp to $DEST ..."
mkdir -p "$DEST"
cp -a "$ANATOMY_SRC/." "$DEST/"

mkdir -p "$DEST/wasm"
for f in qualia_core_db.js qualia_core_db_bg.wasm; do
  if [[ -f "$WASM_SRC/$f" ]]; then
    cp "$WASM_SRC/$f" "$DEST/wasm/"
    echo "  bundled wasm: $f"
  else
    echo "warning: WASM artifact missing: $WASM_SRC/$f" >&2
  fi
done

echo "Bundled qapps staged under $DEST"
