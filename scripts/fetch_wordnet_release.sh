#!/usr/bin/env bash
# Download princeton.q42 from GitHub Release assets (replaces Git LFS).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

VERSION_FILE="$REPO_ROOT/docs/data/wordnet/VERSION"
TAG="v0.0.16"
if [[ -f "$VERSION_FILE" ]]; then
  TAG="v$(tr -d '[:space:]' < "$VERSION_FILE")"
fi

REPO="${QUALIA_GITHUB_REPO:-mediaprophet/qualiaDB}"
OUT_DIR="$REPO_ROOT/docs/data/wordnet"
CANONICAL="$OUT_DIR/princeton.q42"
PLAYGROUND="$REPO_ROOT/docs/playground/wordnet.q42"

mkdir -p "$OUT_DIR"

echo "=== Fetch Princeton WordNet from GitHub Release ==="
echo "  Repo : $REPO"
echo "  Tag  : $TAG"
echo "  Out  : $CANONICAL"
echo ""

if command -v gh &>/dev/null; then
  gh release download "$TAG" --repo "$REPO" --pattern 'princeton.q42' --dir "$OUT_DIR" --clobber
else
  URL="https://github.com/${REPO}/releases/download/${TAG}/princeton.q42"
  curl -fL --progress-bar -o "$CANONICAL" "$URL"
fi

if [[ ! -f "$CANONICAL" ]]; then
  echo "ERROR: princeton.q42 not found after download" >&2
  exit 1
fi

cp "$CANONICAL" "$PLAYGROUND"
ls -lh "$CANONICAL" "$PLAYGROUND"