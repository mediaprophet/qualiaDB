#!/usr/bin/env bash
# Generate releases/latest.json for the in-app updater (upload as release asset + Pages).
set -euo pipefail

VERSION="${1:-0.0.7}"
REPO="${GITHUB_REPOSITORY:-mediaprophet/qualiaDB}"
BASE="https://github.com/${REPO}/releases/download/v${VERSION}"
OUT="${2:-latest.json}"

cat > "$OUT" <<EOF
{
  "version": "${VERSION}",
  "tag": "v${VERSION}",
  "url": "${BASE}",
  "windows_url": "${BASE}/QualiaDB-Setup-${VERSION}-x64.exe",
  "windows_portable_url": "${BASE}/qualia-flutter-windows-x64.zip",
  "macos_url": "${BASE}/QualiaDB-${VERSION}-macos-arm64.dmg",
  "linux_url": "${BASE}/QualiaDB-${VERSION}-linux-x64.deb",
  "linux_portable_url": "${BASE}/qualia-flutter-linux-x64.tar.gz",
  "notes": "QualiaDB Flutter desktop release v${VERSION}"
}
EOF

echo "Wrote $OUT"
cat "$OUT"
