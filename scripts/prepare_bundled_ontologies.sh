#!/usr/bin/env bash
# Ingest bundled TTL ontologies (w3c, purl, …) into .q42 volumes for the Ontology Demo.
set -euo pipefail

GROUP="${1:-w3c}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

case "$GROUP" in
  w3c)
    SRC_DIR="${W3C_TTL_DIR:-$REPO_ROOT/bundled/ontologies/w3c}"
    OUT_DIR="$REPO_ROOT/docs/data/w3c"
    ;;
  purl)
    SRC_DIR="${PURL_TTL_DIR:-$REPO_ROOT/bundled/ontologies/purl}"
    OUT_DIR="$REPO_ROOT/docs/data/purl"
    ;;
  geonames)
    SRC_DIR="${GEONAMES_TTL_DIR:-$REPO_ROOT/bundled/ontologies/geonames}"
    OUT_DIR="$REPO_ROOT/docs/data/geonames"
    ;;
  dublincore)
    SRC_DIR="${DUBLINCORE_TTL_DIR:-$REPO_ROOT/bundled/ontologies/dublincore}"
    OUT_DIR="$REPO_ROOT/docs/data/dublincore"
    ;;
  w3c-archives)
    SRC_DIR="${W3C_ARCHIVES_BUNDLE_DIR:-$REPO_ROOT/bundled/ontologies/w3c-archives}"
    OUT_DIR="$REPO_ROOT/docs/data/w3c-archives"
    ;;
  *)
    echo "Unknown group: $GROUP (expected: w3c, purl, geonames, dublincore, w3c-archives)" >&2
    exit 1
    ;;
esac

CATALOG="$SRC_DIR/catalog.json"
mkdir -p "$OUT_DIR"

if [[ ! -f "$CATALOG" ]]; then
  echo "Missing catalog: $CATALOG" >&2
  exit 1
fi

echo "$GROUP ontology preparation"
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
  base="${file%.*}"
  echo "  ingest: $file -> $base.q42"
  (cd "$REPO_ROOT" && cargo run --release -p qualia-cli -- ingest semantic "$ttl")
  src="$SRC_DIR/$base.q42"
  if [[ -f "$src" ]]; then
    cp "$src" "$OUT_DIR/$base.q42"
  fi
  count=$((count + 1))
done < <(python3 -c "import json,sys; sys.stdout.write('\n'.join(e['file'] for e in json.load(open(sys.argv[1], encoding='utf-8-sig')).get('ontologies',[])))" "$CATALOG")

python3 "$REPO_ROOT/scripts/merge_ontology_manifest.py" "$GROUP"
echo "Sync complete — $count $GROUP ontologies under $OUT_DIR/"