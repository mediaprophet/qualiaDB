#!/usr/bin/env bash
# Build a .deb installer from the Linux portable bundle.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
STAGE="${STAGE_DIR:-$ROOT/dist/qualia-flutter-linux-x64}"
VERSION="${APP_VERSION:-0.0.7}"
PKG_NAME="qualiadb"
DEB="$ROOT/dist/QualiaDB-${VERSION}-linux-x64.deb"
BUILD="$ROOT/dist/deb-build"

if [[ ! -f "$STAGE/qualia_flutter" ]]; then
  echo "Missing staged bundle: $STAGE" >&2
  exit 1
fi

rm -rf "$BUILD"
mkdir -p "$BUILD/DEBIAN"
mkdir -p "$BUILD/opt/qualia/qualiadb"
mkdir -p "$BUILD/usr/share/applications"

cp -a "$STAGE/." "$BUILD/opt/qualia/qualiadb/"

cat > "$BUILD/DEBIAN/control" <<EOF
Package: ${PKG_NAME}
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: amd64
Maintainer: QualiaDB <noreply@qualia.local>
Description: QualiaDB semantic graph desktop (Flutter)
 Native desktop client for the QualiaDB graph engine and agent runtime.
Depends: libgtk-3-0, libsecret-1-0, libjsoncpp25 | libjsoncpp1
EOF

cat > "$BUILD/usr/share/applications/qualiadb.desktop" <<EOF
[Desktop Entry]
Name=QualiaDB
Comment=QualiaDB semantic graph desktop
Exec=/opt/qualia/qualiadb/qualia_flutter
Icon=/opt/qualia/qualiadb/data/flutter_assets/assets/icons/tray_icon.png
Terminal=false
Type=Application
Categories=Development;Science;
EOF

chmod 755 "$BUILD/DEBIAN"
chmod +x "$BUILD/opt/qualia/qualiadb/qualia_flutter"

dpkg-deb --build "$BUILD" "$DEB"
echo "DEB: $DEB"
