#!/usr/bin/env bash
# generate_icons.sh
#
# Regenerates all platform-specific app icons from the master SVG source.
#
# Dependencies (install once):
#   cargo install resvg          # SVG → PNG renderer (pure Rust, no system deps)
#   cargo install tauri-cli --version "^1.5"   # provides `cargo tauri icon`
#
# Usage:
#   bash scripts/generate_icons.sh
#
# Output: crates/qualia-desktop/icons/  — all Tauri icon variants regenerated
#         crates/qualia-desktop/app-icon.png — 1024×1024 master PNG

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DESKTOP_DIR="${REPO_ROOT}/crates/qualia-desktop"
SVG_SRC="${DESKTOP_DIR}/app-icon.svg"
PNG_SRC="${DESKTOP_DIR}/app-icon.png"

echo "=== Qualia-DB Icon Generator ==="
echo "  Source SVG : ${SVG_SRC}"
echo "  Master PNG : ${PNG_SRC}"
echo ""

# ── Step 1: Render SVG → 1024×1024 PNG ──────────────────────────────────────

if ! command -v resvg &>/dev/null; then
  echo "[1/2] Installing resvg..."
  cargo install resvg
fi

echo "[1/2] Rendering SVG → 1024×1024 PNG via resvg..."
resvg --width 1024 --height 1024 "${SVG_SRC}" "${PNG_SRC}"
echo "      Written: ${PNG_SRC}"

# ── Step 2: Generate all Tauri icon variants ─────────────────────────────────

echo "[2/2] Generating platform icons via cargo tauri icon..."
cd "${DESKTOP_DIR}"
cargo tauri icon "${PNG_SRC}"

echo ""
echo "=== Done ==="
echo "Icons written to: ${DESKTOP_DIR}/icons/"
ls -lh "${DESKTOP_DIR}/icons/"
