#!/usr/bin/env bash
# Inventory W3C namespace archives (deduped) and ingest into .q42 volumes.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ARCHIVES_DIR="${W3C_ARCHIVES_DIR:-C:/Projects/ontologies-2023/w3c archives}"
SRC_DIR="$REPO_ROOT/bundled/ontologies/w3c-archives"
OUT_DIR="$REPO_ROOT/docs/data/w3c-archives"
CATALOG="$SRC_DIR/catalog.json"

mkdir -p "$OUT_DIR"

echo "W3C archives ontology preparation"
echo "  Archives dir: $ARCHIVES_DIR"
echo "  Source dir  : $SRC_DIR"
echo "  Output dir  : $OUT_DIR"

if [[ "${W3C_ARCHIVES_SKIP_INVENTORY:-}" == "1" ]]; then
  echo "  skipping inventory (W3C_ARCHIVES_SKIP_INVENTORY=1)"
elif [[ -d "$ARCHIVES_DIR" ]]; then
  python3 "$REPO_ROOT/scripts/inventory_w3c_archives.py" "$ARCHIVES_DIR"
else
  echo "  archives not found — using pre-synced bundled/ontologies/w3c-archives"
fi

if [[ ! -f "$CATALOG" ]]; then
  echo "Missing catalog: $CATALOG" >&2
  exit 1
fi

count=0
while IFS= read -r file; do
  file="${file//$'\r'/}"
  [[ -z "$file" ]] && continue
  src="$SRC_DIR/$file"
  if [[ ! -f "$src" ]] || [[ ! -s "$src" ]]; then
    echo "  skip (missing/empty): $file"
    continue
  fi
  base="${file%.*}"
  echo "  ingest: $file -> $base.q42"
  (cd "$REPO_ROOT" && cargo run --release -p qualia-cli -- ingest semantic "$src")
  artifact="$SRC_DIR/$base.q42"
  if [[ -f "$artifact" ]]; then
    cp "$artifact" "$OUT_DIR/$base.q42"
  fi
  count=$((count + 1))
done < <(python3 -c "import json,sys; sys.stdout.write('\n'.join(e['file'] for e in json.load(open(sys.argv[1], encoding='utf-8-sig')).get('ontologies',[])))" "$CATALOG")

python3 "$REPO_ROOT/scripts/merge_ontology_manifest.py" w3c-archives
echo "Sync complete — $count W3C archive ontologies under $OUT_DIR/"