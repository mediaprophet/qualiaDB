#!/usr/bin/env bash
# Ingest Princeton WordNet 3.1 RDF/XML into unified v3 .q42 and wire demo paths.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

RDF="${QUALIA_PRINCETON_RDF:-}"
if [[ -z "$RDF" ]]; then
  for c in \
    "/c/Projects/ontologies-2023/wordnet.rdf" \
    "$REPO_ROOT/bundled/ontologies/wordnet/wordnet.rdf" \
    "$REPO_ROOT/data/wordnet.rdf"; do
    if [[ -f "$c" ]]; then RDF="$c"; break; fi
  done
fi

if [[ -z "$RDF" || ! -f "$RDF" ]]; then
  echo "Princeton wordnet.rdf not found. Set QUALIA_PRINCETON_RDF=/path/to/wordnet.rdf" >&2
  exit 1
fi

DATA_DIR="$REPO_ROOT/docs/data/wordnet"
PLAYGROUND="$REPO_ROOT/docs/playground/wordnet.q42"
LOCAL_LIB="$REPO_ROOT/Local_LIbraries/wordnet"
CANONICAL="$DATA_DIR/princeton.q42"
BASE="$(basename "${RDF%.*}")"
BUILT_Q42="$(dirname "$RDF")/${BASE}.q42"

mkdir -p "$DATA_DIR" "$LOCAL_LIB"

echo "=== Princeton WordNet → Q42 v3 ==="
echo "  Source : $RDF"
echo "  Output : $CANONICAL"
echo ""

cargo build --release -p qualia-cli --quiet
./target/release/qualia-cli ingest semantic "$RDF"

if [[ ! -f "$BUILT_Q42" ]]; then
  echo "ERROR: ingest did not produce $BUILT_Q42" >&2
  exit 1
fi

cp "$BUILT_Q42" "$CANONICAL"
cp "$BUILT_Q42" "$LOCAL_LIB/wordnet.q42"
cp "$BUILT_Q42" "$PLAYGROUND"

ls -lh "$CANONICAL" "$PLAYGROUND" "$LOCAL_LIB/wordnet.q42"
echo ""
echo "Demos: vfs-manifest.json → data/wordnet/princeton.q42"
echo "Release: tag v* uploads princeton.q42 — or download via scripts/fetch_wordnet_release.sh"