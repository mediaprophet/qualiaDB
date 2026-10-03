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

# Keep dx in lockstep with crates/webizen-studio dioxus "=0.8.0-alpha.1".
DX_CLI_VERSION="${DX_CLI_VERSION:-0.8.0-alpha.1}"
need_dx_install=1
if command -v dx >/dev/null 2>&1; then
  # `dx --version` prints e.g. "dioxus 0.8.0-alpha.1" (or dx/dioxus-cli variants).
  if dx --version 2>/dev/null | grep -Eq "${DX_CLI_VERSION}"; then
    need_dx_install=0
  fi
fi
if [[ "$need_dx_install" -eq 1 ]]; then
  echo "Installing dioxus-cli ${DX_CLI_VERSION} (must match studio dioxus pin)..."
  cargo install dioxus-cli --version "${DX_CLI_VERSION}" --locked --force
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

# NO_DOWNLOADS requires esbuild and wasm-opt already on PATH.
ESBUILD_VERSION="${ESBUILD_VERSION:-0.27.3}"
esbuild_ok=0
if command -v esbuild >/dev/null 2>&1; then
  if esbuild --version 2>/dev/null | grep -Eq "${ESBUILD_VERSION}"; then
    esbuild_ok=1
  fi
fi
if [[ "$esbuild_ok" -eq 0 ]]; then
  if command -v npm >/dev/null 2>&1; then
    echo "Installing esbuild ${ESBUILD_VERSION}..."
    npm install --global "esbuild@${ESBUILD_VERSION}" || true
  fi
fi

# Ensure wasm-opt exists on PATH
if ! command -v wasm-opt >/dev/null 2>&1; then
  BINARYEN_VERSION="${BINARYEN_VERSION:-123}"
  dx_tools="$HOME/.dx/tools"
  binaryen_home="$dx_tools/binaryen-version_$BINARYEN_VERSION"
  if [[ ! -x "$binaryen_home/bin/wasm-opt" ]]; then
    mkdir -p "$dx_tools"
    arch="x86_64"
    if [[ "$(uname -m)" == "arm64" || "$(uname -m)" == "aarch64" ]]; then
      arch="arm64"
    fi
    os_tag="linux"
    if [[ "$(uname -s)" == "Darwin" ]]; then
      os_tag="macos"
    fi
    asset="binaryen-version_${BINARYEN_VERSION}-${arch}-${os_tag}.tar.gz"
    url="https://github.com/WebAssembly/binaryen/releases/download/version_${BINARYEN_VERSION}/${asset}"
    echo "Downloading wasm-opt from $url..."
    if curl -sSL -o "$dx_tools/$asset" "$url"; then
      tar -xzf "$dx_tools/$asset" -C "$dx_tools" || true
      rm -f "$dx_tools/$asset"
    fi
  fi
  if [[ -x "$binaryen_home/bin/wasm-opt" ]]; then
    export PATH="$binaryen_home/bin:$PATH"
  fi
fi

# Dioxus otherwise ignores the verified PATH binary and attempts to
# redownload a managed wasm-bindgen on every invocation.
export NO_DOWNLOADS=1
# Host-cpu RUSTFLAGS (e.g. -C target-cpu=apple-m1) break wasm32 + wasm-bindgen.
# Also disable wasm fat-LTO / bitcode: rust-lld fails with
# rust-lld rejects duplicate symbols when multiple definitions are introduced;
# dynamic reflection in tauri_ffi avoids duplicate externs entirely.
FRONTEND_WASM_RUSTFLAGS="-C lto=off -C embed-bitcode=no"
export RUSTFLAGS="${RUSTFLAGS_WASM:-$FRONTEND_WASM_RUSTFLAGS}"
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-$FRONTEND_WASM_RUSTFLAGS}"
# dx --release uses web-release; also clear release/wasm-release LTO env overrides Capt may set.
export CARGO_PROFILE_WEB_RELEASE_LTO="${CARGO_PROFILE_WEB_RELEASE_LTO:-false}"
export CARGO_PROFILE_RELEASE_LTO="${CARGO_PROFILE_RELEASE_LTO:-false}"
export CARGO_PROFILE_WASM_RELEASE_LTO="${CARGO_PROFILE_WASM_RELEASE_LTO:-false}"
export CARGO_INCREMENTAL=0
unset CARGO_ENCODED_RUSTFLAGS || true

(
  cd "$repo_root/crates/webizen-studio"
  # Drop stale dx artifacts so mixed bitcode / duplicate wbindgen objects cannot relink.
  rm -rf "$repo_root/target/dx/webizen-studio" || true
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
