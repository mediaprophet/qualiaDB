#!/usr/bin/env bash
# Create macOS .dmg from staged qualia_flutter.app (Apple Silicon).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
APP_DIR="${STAGE_DIR:-$ROOT/dist/qualia-flutter-macos-arm64/qualia_flutter.app}"
OUT_DIR="$ROOT/dist"
VERSION="${APP_VERSION:-0.0.7}"
DMG="$OUT_DIR/QualiaDB-${VERSION}-macos-arm64.dmg"
STAGING="$OUT_DIR/dmg-staging"

if [[ ! -d "$APP_DIR" ]]; then
  echo "Missing app bundle: $APP_DIR" >&2
  exit 1
fi

rm -rf "$STAGING" "$DMG"
mkdir -p "$STAGING"
cp -R "$APP_DIR" "$STAGING/"
ln -s /Applications "$STAGING/Applications"

hdiutil create -volname "QualiaDB" -srcfolder "$STAGING" -ov -format UDZO "$DMG"
rm -rf "$STAGING"
echo "DMG: $DMG"
