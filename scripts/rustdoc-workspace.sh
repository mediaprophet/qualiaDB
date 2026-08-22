#!/usr/bin/env bash
# Document every workspace *library* crate that does not need a desktop/Tauri host.
# Used by Pages CI and local `bash scripts/rustdoc-workspace.sh`.
set -euo pipefail
cd "$(dirname "$0")/.."

# webizen-desktop: Tauri host (docs via manuals, not rustdoc on Pages).
# webizen-component-harvester: one-shot bin.
# webizen-studio: huge Dioxus app — still a lib; include it so UI types are findable.
cargo doc --no-deps --workspace \
  --exclude webizen-desktop \
  --exclude webizen-component-harvester

echo "rustdoc crates written under target/doc/"
ls -1 target/doc | grep -E '^(qualia_|poet_|webizen_|wellfare_)' || true
