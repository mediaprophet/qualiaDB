#!/usr/bin/env bash
# Pull anatomy .hmc packs (+ .q42 provenance) from the GitHub Release into
# docs/playground/ so Pages deploys same-origin assets for anatomy.html.
#
# Prefer the canonical engine version from crates/qualia-core-db/Cargo.toml so
# we do not pin an old tag (e.g. v0.0.33) after a release bump.
#
# Pages can start before the Anatomy Asset Packs workflow has uploaded this
# tag's assets. Probe with HEAD first (no 404 body spam), retry the preferred
# tag briefly, then fall back to older anatomy-bearing releases.
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
PREFERRED_TAG="${ANATOMY_RELEASE_TAG:-${PREFERRED_TAG:-v0.0.33}}"

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

# Unique fallbacks only. Duplicate v0.0.33 entries used to 404-spam CI.
FALLBACK_TAGS=("$PREFERRED_TAG")
for tag in v0.0.36 v0.0.35 v0.0.33; do
  if [[ "$tag" != "$PREFERRED_TAG" ]]; then
    FALLBACK_TAGS+=("$tag")
  fi
done

echo "=== Fetch anatomy packs from GitHub Release ==="
echo "  Repo : $REPO"
echo "  Prefer tag : $PREFERRED_TAG"
echo "  Out  : $OUT_DIR"
echo ""

asset_url() {
  local tag="$1"
  local name="$2"
  printf 'https://github.com/%s/releases/download/%s/%s' "$REPO" "$tag" "$name"
}

# HEAD-probe so missing tags do not emit curl (22) 404 on the download URL.
asset_exists() {
  local url="$1"
  local code
  code="$(curl -sI -o /dev/null -w '%{http_code}' --retry 2 --retry-delay 1 "$url" || true)"
  case "$code" in
    200|301|302|303|307|308) return 0 ;;
    *) return 1 ;;
  esac
}

# Binary-safe: never command-substitute pack bytes (NUL in .hmc trips bash).
reject_html_or_empty() {
  local dest="$1"
  python3 - "$dest" <<'PY'
import sys
from pathlib import Path
p = Path(sys.argv[1])
if not p.is_file() or p.stat().st_size == 0:
    sys.exit(1)
head = p.read_bytes()[:64].lstrip().lower()
if head.startswith((b"<!doctype", b"<html")):
    sys.exit(1)
sys.exit(0)
PY
}

download_asset() {
  local tag="$1"
  local name="$2"
  local dest="$OUT_DIR/$name"
  local url
  url="$(asset_url "$tag" "$name")"
  if ! asset_exists "$url"; then
    echo "  miss ${name} ← ${tag} (not on this release)"
    return 1
  fi
  if curl -fL --retry 3 --retry-delay 2 --progress-bar -o "$dest" "$url"; then
    if reject_html_or_empty "$dest"; then
      echo "  ok  ${name} ← ${tag} ($(du -h "$dest" | cut -f1))"
      return 0
    fi
  fi
  rm -f "$dest"
  return 1
}

try_tag() {
  local tag="$1"
  local name
  echo "Trying release ${tag}…"
  for name in "${REQUIRED[@]}"; do
    if ! download_asset "$tag" "$name"; then
      for name in "${REQUIRED[@]}"; do
        rm -f "$OUT_DIR/$name"
      done
      return 1
    fi
  done
  return 0
}

TAG_USED=""
# Race: Pages often starts before anatomy-pack assets exist on this tag.
if try_tag "$PREFERRED_TAG"; then
  TAG_USED="$PREFERRED_TAG"
else
  for _try in 1 2 3; do
    echo "Preferred tag ${PREFERRED_TAG} not ready — retry ${_try}/3 in 20s (anatomy workflow may still be uploading)…"
    sleep 20
    if try_tag "$PREFERRED_TAG"; then
      TAG_USED="$PREFERRED_TAG"
      break
    fi
  done
fi

if [[ -z "$TAG_USED" ]]; then
  for TAG in "${FALLBACK_TAGS[@]}"; do
    if [[ "$TAG" == "$PREFERRED_TAG" ]]; then
      continue
    fi
    if try_tag "$TAG"; then
      TAG_USED="$TAG"
      break
    fi
  done
fi

if [[ -z "$TAG_USED" ]]; then
  echo "ERROR: required anatomy packs not found on releases: ${FALLBACK_TAGS[*]}" >&2
  exit 1
fi

for name in "${OPTIONAL[@]}"; do
  download_asset "$TAG_USED" "$name" || echo "  skip optional ${name}"
done

ls -lh "$OUT_DIR"/anatomy-male.hmc "$OUT_DIR"/anatomy-female.hmc
echo "Anatomy packs ready from release ${TAG_USED}"
