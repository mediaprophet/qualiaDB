#!/usr/bin/env bash
# Prepare Schema.org benchmark artifacts for the comparative harness (Linux/macOS CI).
set -euo pipefail

RELEASE="${1:-30.0}"
VARIANT="${2:-current-https}"
DATA_ROOT="${DATA_ROOT:-data/schemaorg}"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RELEASE_DIR="$REPO_ROOT/$DATA_ROOT/$RELEASE"
DOCS_RELEASE_DIR="$REPO_ROOT/docs/data/schemaorg/$RELEASE"
BASE_NAME="schemaorg-$VARIANT"
NT_PATH="$RELEASE_DIR/$BASE_NAME.nt"
Q42_BASE="$RELEASE_DIR/$BASE_NAME"
Q42_PATH="$Q42_BASE.q42"
CQ42_PATH="$Q42_BASE.c.q42"
RAW_URL="https://raw.githubusercontent.com/schemaorg/schemaorg/main/data/releases/$RELEASE/$BASE_NAME.nt"

mkdir -p "$RELEASE_DIR"
mkdir -p "$DOCS_RELEASE_DIR"

echo "Schema.org benchmark preparation"
echo "  Source URL : $RAW_URL"
echo "  NT output  : $NT_PATH"
echo "  Q42 output : $Q42_PATH"

if [[ ! -f "$NT_PATH" ]]; then
  echo "Downloading Schema.org release file..."
  curl -sSfL "$RAW_URL" -o "$NT_PATH"
else
  echo "NT source already present, reusing local file."
fi

echo "Ingesting N-Triples into native .q42..."
(cd "$REPO_ROOT" && cargo run --release -p qualia-cli -- ingest --input "$NT_PATH" --output "$Q42_BASE")

echo "Compressing .q42 artifact for distribution..."
(cd "$REPO_ROOT" && cargo run --release -p qualia-cli -- compress --input "$Q42_PATH" --output "$CQ42_PATH")

echo "Syncing benchmark artifacts into docs/data for GitHub Pages and local site testing..."
cp "$NT_PATH" "$DOCS_RELEASE_DIR/"
cp "$Q42_PATH" "$DOCS_RELEASE_DIR/"
cp "$CQ42_PATH" "$DOCS_RELEASE_DIR/"
if [[ -f "$Q42_PATH.bidx" ]]; then
  cp "$Q42_PATH.bidx" "$DOCS_RELEASE_DIR/"
fi
if [[ -f "$Q42_PATH.lex" ]]; then
  cp "$Q42_PATH.lex" "$DOCS_RELEASE_DIR/"
fi

echo "Done. Run:"
echo "  python benchmarks/harness.py --all --dataset-profile schemaorg-30-current-https --output docs/comparative_benchmark_results.schemaorg-30-current-https.json"
