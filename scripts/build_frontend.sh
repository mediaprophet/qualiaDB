#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
public="$repo_root/target/dx/webizen-studio/release/web/public"
dist="$repo_root/crates/webizen-studio/dist"
assets="$dist/assets"
browser_source="$repo_root/crates/webizen-desktop/src/browser"
# Keep in lockstep with crates/webizen-studio/Cargo.toml wasm-bindgen pin.
WASM_BINDGEN_CLI_VERSION="${WASM_BINDGEN_CLI_VERSION:-0.2.125}"
source_revision="$(git -C "$repo_root" rev-parse HEAD)"
if [[ -n "$(git -C "$repo_root" status --porcelain)" ]]; then
  source_revision="${source_revision}-dirty"
fi

if ! command -v dx >/dev/null 2>&1; then
  cargo install dioxus-cli --version 0.8.0-alpha.0 --locked
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
echo "Using $(command -v wasm-bindgen): $(wasm-bindgen --version)"
# Host-cpu RUSTFLAGS (e.g. -C target-cpu=apple-m1) break wasm32 + wasm-bindgen.
# Clear for the frontend build only.
export RUSTFLAGS="${RUSTFLAGS_WASM:-}"
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-}"
unset CARGO_ENCODED_RUSTFLAGS || true

(
  cd "$repo_root/crates/webizen-studio"
  # Prefer a clean bindgen output dir so a previous failed mac run cannot leave a half file.
  rm -rf "$repo_root/target/dx/webizen-studio/release/web/public/wasm" || true
  dx build --web --release
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
