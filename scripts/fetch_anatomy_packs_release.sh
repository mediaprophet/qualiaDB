#!/usr/bin/env bash
# Pull anatomy .hmc packs (+ .q42 provenance) from the GitHub Release into
# docs/playground/ so Pages deploys same-origin assets for anatomy.html.
#
# Prefer the canonical engine version from crates/qualia-core-db/Cargo.toml so
# we do not pin an old tag (e.g. v0.0.24) after a release bump.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

CARGO_TOML="$REPO_ROOT/crates/qualia-core-db/Cargo.toml"
PREFERRED_TAG=""
if [[ -f "$CARGO_TOML" ]]; then
  VER="$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\([0-9.]*\)".*/\1/p' "$CARGO_TOML" | head -n1)"
  if [[ -n "$VER" ]]; then
    PREFERRED_TAG="v${VER}"
  fi
fi
PREFERRED_TAG="${ANATOMY_RELEASE_TAG:-${PREFERRED_TAG:-v0.0.30}}"

REPO="${QUALIA_GITHUB_REPO:-mediaprophet/qualiaDB}"
OUT_DIR="$REPO_ROOT/docs/playground"
mkdir -p "$OUT_DIR"

# Male/female CCF packs are required for the online demo (same-origin fetch).
# BodyParts3D complete pack is optional here (too large for many Pages budgets;
# the demo offers a manual file-load path for it).
REQUIRED=(
  "anatomy-male.hmc"
  "anatomy-female.hmc"
)
OPTIONAL=(
  "anatomy-male.q42"
  "anatomy-female.q42"
  "anatomy-bodyparts3d.hmc"
  "anatomy-bodyparts3d.q42"
)

# Prefer current tag, then recent known anatomy-bearing releases.
FALLBACK_TAGS=("$PREFERRED_TAG" "v0.0.29" "v0.0.28" "v0.0.27" "v0.0.26" "v0.0.24")

echo "=== Fetch anatomy packs from GitHub Release ==="
echo "  Repo : $REPO"
echo "  Prefer tag : $PREFERRED_TAG"
echo "  Out  : $OUT_DIR"
echo ""

download_asset() {
  local tag="$1"
  local name="$2"
  local dest="$OUT_DIR/$name"
  local url="https://github.com/${REPO}/releases/download/${tag}/${name}"
  if curl -fL --retry 3 --retry-delay 2 --progress-bar -o "$dest" "$url"; then
    # Reject empty / HTML error pages disguised as success
    if [[ ! -s "$dest" ]]; then
      rm -f "$dest"
      return 1
    fi
    local head
    head="$(head -c 15 "$dest" 2>/dev/null || true)"
    if [[ "$head" == *"<!DOCTYPE"* ]] || [[ "$head" == *"<html"* ]]; then
      rm -f "$dest"
      return 1
    fi
    echo "  ok  ${name} ← ${tag} ($(du -h "$dest" | cut -f1))"
    return 0
  fi
  rm -f "$dest"
  return 1
}

TAG_USED=""
for TAG in "${FALLBACK_TAGS[@]}"; do
  echo "Trying release ${TAG}…"
  ok=true
  for name in "${REQUIRED[@]}"; do
    if ! download_asset "$TAG" "$name"; then
      ok=false
      break
    fi
  done
  if [[ "$ok" == "true" ]]; then
    TAG_USED="$TAG"
    break
  fi
  # Clean partial required downloads before trying the next tag
  for name in "${REQUIRED[@]}"; do
    rm -f "$OUT_DIR/$name"
  done
done

if [[ -z "$TAG_USED" ]]; then
  echo "ERROR: required anatomy packs not found on releases: ${FALLBACK_TAGS[*]}" >&2
  exit 1
fi

for name in "${OPTIONAL[@]}"; do
  download_asset "$TAG_USED" "$name" || echo "  skip optional ${name}"
done

ls -lh "$OUT_DIR"/anatomy-male.hmc "$OUT_DIR"/anatomy-female.hmc
echo "Anatomy packs ready from release ${TAG_USED}"
