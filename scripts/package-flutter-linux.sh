#!/usr/bin/env bash
# Package QualiaDB Flutter desktop for Linux x64 (portable bundle under dist/).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FLUTTER="$ROOT/crates/qualia-flutter"
BUNDLE="$FLUTTER/build/linux/x64/release/bundle"
OUT="${OUT_DIR:-$ROOT/dist/qualia-flutter-linux-x64}"
RUST_LIB="$ROOT/target/release/libqualia_flutter_rust.so"

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  echo "Building Flutter Linux release..."
  (cd "$FLUTTER" && flutter pub get && flutter build linux --release)
  echo "Building Rust FFI (release)..."
  (cd "$ROOT" && cargo build --release -p qualia_flutter_rust)
fi

if [[ ! -d "$BUNDLE" ]]; then
  echo "Missing Flutter bundle: $BUNDLE" >&2
  exit 1
fi
if [[ ! -f "$RUST_LIB" ]]; then
  echo "Missing Rust library: $RUST_LIB" >&2
  exit 1
fi

echo "Staging portable bundle to $OUT ..."
rm -rf "$OUT"
mkdir -p "$OUT"
cp -a "$BUNDLE/." "$OUT/"
cp "$RUST_LIB" "$OUT/lib/"
chmod +x "$OUT/qualia_flutter"
"$ROOT/scripts/copy-bundled-qapps.sh" "$OUT"
echo "Done. Portable bundle: $OUT/qualia_flutter"
