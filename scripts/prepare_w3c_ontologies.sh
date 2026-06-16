#!/usr/bin/env bash
# Ingest bundled W3C TTL ontologies into .q42 volumes for the Ontology Demo.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC_DIR="${W3C_TTL_DIR:-$REPO_ROOT/bundled/ontologies/w3c}"
CATALOG="$SRC_DIR/catalog.json"
OUT_DIR="$REPO_ROOT/docs/data/w3c"
MANIFEST="$REPO_ROOT/docs/playground/vfs-manifest.json"

mkdir -p "$OUT_DIR"

if [[ ! -f "$CATALOG" ]]; then
  echo "Missing catalog: $CATALOG" >&2
  exit 1
fi

echo "W3C ontology preparation"
echo "  Source dir : $SRC_DIR"
echo "  Output dir : $OUT_DIR"

count=0
while IFS= read -r file; do
  file="${file//$'\r'/}"
  [[ -z "$file" ]] && continue
  ttl="$SRC_DIR/$file"
  if [[ ! -f "$ttl" ]] || [[ ! -s "$ttl" ]]; then
    echo "  skip (missing/empty): $file"
    continue
  fi
  base="${file%.ttl}"
  echo "  ingest: $file -> $base.q42"
  (cd "$REPO_ROOT" && cargo run --release -p qualia-cli -- ingest semantic "$ttl")
  src="$SRC_DIR/$base.q42"
  if [[ -f "$src" ]]; then
    cp "$src" "$OUT_DIR/$base.q42"
  fi
  count=$((count + 1))
done < <(python3 -c "import json,sys; sys.stdout.write('\n'.join(e['file'] for e in json.load(open(sys.argv[1])).get('ontologies',[])))" "$CATALOG")

python3 "$REPO_ROOT/scripts/merge_ontology_manifest.py" w3c
echo "Sync complete — $count W3C ontologies under docs/data/w3c/"