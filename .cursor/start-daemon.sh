#!/usr/bin/env bash
# Long-running terminal: the Qualia native loopback daemon (HTTP graph/SPARQL
# engine on http://127.0.0.1:4242, endpoints /health and /query).
#
# The daemon's KeyVault reads its master signing key from the OS keyring. On a
# headless VM there is no Secret Service, so we run inside a private D-Bus
# session with an unlocked gnome-keyring. `--dev` trusts localhost origins.
set -euo pipefail

cd "$(dirname "$0")/.."

export QUALIA_DATA_DIR="${QUALIA_DATA_DIR:-$HOME/.qualia-daemon}"
mkdir -p "$QUALIA_DATA_DIR"

QUALIA_CLI_BIN="$PWD/target/release/qualia-cli"
if [ ! -x "$QUALIA_CLI_BIN" ]; then
  echo "start-daemon.sh: building qualia-cli (release) first..."
  cargo build --release -p qualia-cli
fi
export QUALIA_CLI_BIN

exec dbus-run-session -- bash -c '
  # Create/unlock a login keyring with an empty passphrase and start the
  # Secret Service component on the session bus.
  eval "$(printf "\n" | gnome-keyring-daemon --daemonize --login 2>/dev/null)" || true
  eval "$(gnome-keyring-daemon --start --components=secrets 2>/dev/null)" || true
  export GNOME_KEYRING_CONTROL SSH_AUTH_SOCK
  exec "$QUALIA_CLI_BIN" daemon serve --dev --port 4242
'
