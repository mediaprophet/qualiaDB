#!/usr/bin/env bash
# Download princeton.q42 from GitHub Release assets (replaces Git LFS).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

VERSION_FILE="$REPO_ROOT/docs/data/wordnet/VERSION"
PREFERRED_TAG="v0.0.16"
if [[ -f "$VERSION_FILE" ]]; then
  PREFERRED_TAG="v$(tr -d '[:space:]' < "$VERSION_FILE")"
fi

REPO="${QUALIA_GITHUB_REPO:-mediaprophet/qualiaDB}"
OUT_DIR="$REPO_ROOT/docs/data/wordnet"
CANONICAL="$OUT_DIR/princeton.q42"
PLAYGROUND="$REPO_ROOT/docs/playground/wordnet.q42"

mkdir -p "$OUT_DIR"

echo "=== Fetch Princeton WordNet from GitHub Release ==="
echo "  Repo : $REPO"
echo "  Prefer tag : $PREFERRED_TAG"
echo "  Out  : $CANONICAL"
echo ""

# Dataset asset is only published when WORDNET_RDF_URL is set at release time.
# Fall back to older tags that still ship princeton.q42.
FALLBACK_TAGS=("$PREFERRED_TAG" "v0.0.16" "v0.0.15" "v0.0.14" "v0.0.12")
DOWNLOADED=false
for TAG in "${FALLBACK_TAGS[@]}"; do
  URL="https://github.com/${REPO}/releases/download/${TAG}/princeton.q42"
  echo "Trying ${TAG}…"
  if curl -fL --progress-bar -o "$CANONICAL" "$URL"; then
    echo "Downloaded princeton.q42 from release ${TAG}"
    DOWNLOADED=true
    break
  fi
done

if [[ "$DOWNLOADED" != "true" ]] && command -v gh &>/dev/null && [[ -n "${GH_TOKEN:-${GITHUB_TOKEN:-}}" ]]; then
  for TAG in "${FALLBACK_TAGS[@]}"; do
    echo "Trying gh release download ${TAG}…"
    if gh release download "$TAG" --repo "$REPO" --pattern 'princeton.q42' --dir "$OUT_DIR" --clobber 2>/dev/null; then
      DOWNLOADED=true
      break
    fi
  done
fi

if [[ "$DOWNLOADED" != "true" ]]; then
  echo "ERROR: princeton.q42 not found on releases: ${FALLBACK_TAGS[*]}" >&2
  exit 1
fi

if [[ ! -f "$CANONICAL" ]]; then
  echo "ERROR: princeton.q42 not found after download" >&2
  exit 1
fi

cp "$CANONICAL" "$PLAYGROUND"
ls -lh "$CANONICAL" "$PLAYGROUND"