#!/usr/bin/env bash
# Package QualiaDB Flutter desktop for macOS Apple Silicon (arm64).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FLUTTER="$ROOT/crates/qualia-flutter"
APP_SRC="$FLUTTER/build/macos/Build/Products/Release/qualia_flutter.app"
OUT="${OUT_DIR:-$ROOT/dist/qualia-flutter-macos-arm64}"
RUST_LIB="$ROOT/target/release/libqualia_flutter_rust.dylib"

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  echo "Building Flutter macOS release..."
  (cd "$FLUTTER" && flutter pub get && flutter build macos --release)
  echo "Building Rust FFI (release)..."
  (cd "$ROOT" && cargo build --release -p qualia_flutter_rust)
fi

if [[ ! -d "$APP_SRC" ]]; then
  echo "Missing .app bundle: $APP_SRC" >&2
  exit 1
fi
if [[ ! -f "$RUST_LIB" ]]; then
  echo "Missing Rust library: $RUST_LIB" >&2
  exit 1
fi

echo "Staging .app to $OUT ..."
rm -rf "$OUT"
mkdir -p "$OUT"
cp -R "$APP_SRC" "$OUT/"
cp "$RUST_LIB" "$OUT/qualia_flutter.app/Contents/MacOS/"
"$ROOT/scripts/copy-bundled-qapps.sh" "$OUT/qualia_flutter.app/Contents/MacOS"
"$ROOT/scripts/copy-bundled-resources.sh" "$OUT/qualia_flutter.app/Contents/MacOS"
echo "Done. App bundle: $OUT/qualia_flutter.app"
