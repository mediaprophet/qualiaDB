#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
public="$repo_root/target/dx/webizen-studio/release/web/public"
dist="$repo_root/crates/webizen-studio/dist"
assets="$dist/assets"
browser_source="$repo_root/crates/webizen-desktop/src/browser"
source_revision="$(git -C "$repo_root" rev-parse HEAD)"
if [[ -n "$(git -C "$repo_root" status --porcelain)" ]]; then
  source_revision="${source_revision}-dirty"
fi

if ! command -v dx >/dev/null 2>&1; then
  cargo install dioxus-cli --version 0.8.0-alpha.0 --locked
fi

(
  cd "$repo_root/crates/webizen-studio"
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
