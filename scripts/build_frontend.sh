#!/usr/bin/env bash
# Rebuild Webizen Studio web assets for Desktop / Capt re-UAT.
# One command from the repository root:
#   bash scripts/build_frontend.sh
# Stages crates/webizen-studio/dist and writes source-revision.txt at HEAD.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
public="$repo_root/target/dx/webizen-studio/release/web/public"
dist="$repo_root/crates/webizen-studio/dist"
assets="$dist/assets"
browser_source="$repo_root/crates/webizen-desktop/src/browser"
# Keep in lockstep with crates/webizen-studio/Cargo.toml dioxus / wasm-bindgen pins.
DIOXUS_CLI_VERSION="${DIOXUS_CLI_VERSION:-0.8.0-alpha.1}"
WASM_BINDGEN_CLI_VERSION="${WASM_BINDGEN_CLI_VERSION:-0.2.125}"
source_revision="$(git -C "$repo_root" rev-parse HEAD)"
if [[ -n "$(git -C "$repo_root" status --porcelain)" ]]; then
  source_revision="${source_revision}-dirty"
fi

need_dx_install=1
if command -v dx >/dev/null 2>&1; then
  dx_ver="$(dx --version 2>/dev/null || true)"
  if [[ "$dx_ver" == *"${DIOXUS_CLI_VERSION}"* ]]; then
    need_dx_install=0
  fi
fi
if [[ "$need_dx_install" -eq 1 ]]; then
  echo "Installing dioxus-cli ${DIOXUS_CLI_VERSION} (must match crates/webizen-studio dioxus pin)..."
  cargo install dioxus-cli --version "${DIOXUS_CLI_VERSION}" --locked --force
fi

# dx shell-outs to whatever `wasm-bindgen` is on PATH. A CLI/crate mismatch fails with:
#   failed to find the `__wbindgen_externref_table_dealloc` function
# (seen on macOS GitHub runners). Force the matching CLI before building.
need_wb_install=1
if command -v wasm-bindgen >/dev/null 2>&1; then
  if wasm-bindgen --version 2>/dev/null | grep -q "wasm-bindgen ${WASM_BINDGEN_CLI_VERSION}"; then
    need_wb_install=0
  fi
fi
if [[ "$need_wb_install" -eq 1 ]]; then
  echo "Installing wasm-bindgen-cli ${WASM_BINDGEN_CLI_VERSION} (must match crate pin)..."
  cargo install wasm-bindgen-cli --version "${WASM_BINDGEN_CLI_VERSION}" --locked --force
fi
# Prefer cargo-installed tools over any host/Homebrew wasm-bindgen.
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
echo "Using $(command -v dx): $(dx --version 2>/dev/null || true)"
echo "Using $(command -v wasm-bindgen): $(wasm-bindgen --version)"
# Host-cpu RUSTFLAGS (e.g. -C target-cpu=apple-m1) break wasm32 + wasm-bindgen.
# Disable wasm fat-LTO / bitcode: rustc fails at link with
#   failed to load bitcode of module "webizen_studio-*.rcgu.o"
# when dx uses the workspace wasm-release profile (LTO=true) or leftover
# objects remain. Identical Tauri wasm-bindgen `listen`/`invoke` imports in
# the studio bin + lib then become hard rust-lld duplicates without LTO;
# allow the identical first definition (same describe hash).
FRONTEND_WASM_RUSTFLAGS="-C lto=off -C embed-bitcode=no -C link-arg=--allow-multiple-definition"
export RUSTFLAGS="${RUSTFLAGS_WASM:-$FRONTEND_WASM_RUSTFLAGS}"
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-$FRONTEND_WASM_RUSTFLAGS}"
export CARGO_PROFILE_WEB_RELEASE_LTO="${CARGO_PROFILE_WEB_RELEASE_LTO:-off}"
export CARGO_PROFILE_WASM_RELEASE_LTO="${CARGO_PROFILE_WASM_RELEASE_LTO:-off}"
export CARGO_INCREMENTAL=0
# Never inherit a `cargo install` scratch dir (CARGO_BUILD_BUILD_DIR) or a
# foreign CARGO_TARGET_DIR — those mix bitcode objects across profiles.
unset CARGO_ENCODED_RUSTFLAGS CARGO_BUILD_BUILD_DIR CARGO_TARGET_DIR || true

(
  cd "$repo_root/crates/webizen-studio"
  # Drop stale dx artifacts so mixed bitcode objects cannot relink.
  rm -rf "$repo_root/target/dx/webizen-studio" || true
  # --profile web-release: do not use dx's default name `wasm-release`
  # (workspace LTO=true profile for mobile-harness / Windows wasm-opt).
  dx build --web --release --profile web-release
)

test -f "$public/index.html"
mkdir -p "$assets"
find "$assets" -maxdepth 1 -type f -name 'webizen-studio*' -delete
cp "$public/index.html" "$dist/index.html"
cp -R "$public/assets/." "$assets/"
cp "$browser_source/chrome.html" "$dist/browser-chrome.html"
cp "$browser_source/universe.html" "$dist/chora-universe.html"
printf '%s' "$source_revision" > "$dist/source-revision.txt"

echo "Build complete. Staged fresh desktop assets in crates/webizen-studio/dist."
