#!/usr/bin/env bash
# Copy resources/ YAML catalog into a desktop dist folder.
# Usage: ./scripts/copy-bundled-resources.sh dist/qualia-flutter-linux-x64

set -euo pipefail
OUT_DIR="${1:?usage: copy-bundled-resources.sh <out-dir>}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/resources"
DEST="$OUT_DIR/bundled/resources"
ONTOLOGY_SRC="$ROOT/bundled/ontologies"
ONTOLOGY_DEST="$OUT_DIR/bundled/ontologies"

if [[ ! -d "$SRC" ]]; then
  echo "resources/ not found at $SRC — skipping bundled resources copy." >&2
  exit 0
fi

mkdir -p "$DEST"
cp -a "$SRC/." "$DEST/"
echo "Bundled resources staged under $DEST"

if [[ -d "$ONTOLOGY_SRC" ]]; then
  mkdir -p "$ONTOLOGY_DEST"
  cp -a "$ONTOLOGY_SRC/." "$ONTOLOGY_DEST/"
  echo "Bundled ontology sources staged under $ONTOLOGY_DEST"
fi
