#!/usr/bin/env bash
# Idempotent Cloud Agent setup for QualiaDB / Webizen.
#
# Prepares the native build + daemon runtime:
#   * OpenSSL / pkg-config     – required to compile the CLI dependency tree.
#   * gnome-keyring + dbus     – the daemon's KeyVault uses the OS Secret Service
#                                (keyring crate); headless Linux has none by
#                                default, so the daemon panics without it.
#   * Rust >= 1.85             – the workspace uses edition2024.
# Finishes by building the primary application (`qualia-cli`, which also hosts
# the loopback daemon) in release mode.
set -euo pipefail

cd "$(dirname "$0")/.."

# 1) System packages (only touch apt when something is actually missing).
need_pkgs=()
for pkg in libssl-dev pkg-config gnome-keyring dbus dbus-x11; do
  dpkg -s "$pkg" >/dev/null 2>&1 || need_pkgs+=("$pkg")
done
if [ "${#need_pkgs[@]}" -gt 0 ]; then
  sudo apt-get update -qq
  sudo apt-get install -y --no-install-recommends "${need_pkgs[@]}"
fi

# 2) Rust toolchain: ensure a modern stable (workspace requires edition2024).
if command -v rustup >/dev/null 2>&1; then
  rustup toolchain install stable --profile minimal --no-self-update
  rustup default stable
fi
rustc --version

# 3) Build the primary application (native CLI + in-process HTTP daemon).
cargo build --release -p qualia-cli

echo "install.sh: qualia-cli built at target/release/qualia-cli"
