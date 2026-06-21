#!/usr/bin/env bash
# Scrape a person's public W3C mailing-list posts -> RDF -> self-owned .q42 vault.
#   ./scripts/build_w3c_mail_q42.sh [email] [--bodies]
# Defaults to timothy.holborn@gmail.com. Pass --bodies to also archive full message bodies
# (slower, but resumable via the on-disk cache). Output lands in ./provenance/.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

EMAIL="${1:-timothy.holborn@gmail.com}"
shift || true
EXTRA=("$@")   # e.g. --bodies

OUTDIR="$REPO_ROOT/provenance"
STEM="$OUTDIR/w3c_$(echo "$EMAIL" | sed 's/@.*//; s/\./_/g')"
mkdir -p "$OUTDIR"

echo "=== W3C mail -> .q42 provenance vault for $EMAIL ==="

# 1. scrape -> RDF (.ttl human-readable + .nt lossless for ingest)
python3 scripts/scrape_w3c_mail_to_rdf.py --email "$EMAIL" --out "$STEM.ttl" --format both "${EXTRA[@]}"

# 2. build the CLI if needed
if [[ ! -x "$REPO_ROOT/target/release/qualia-cli" && ! -x "$REPO_ROOT/target/release/qualia-cli.exe" ]]; then
  echo "Building qualia-cli (release) ..."
  cargo build --release -p qualia-cli --quiet
fi
CLI="$REPO_ROOT/target/release/qualia-cli"
[[ -x "$CLI.exe" ]] && CLI="$CLI.exe"

# 3. ingest the N-Triples (lossless) -> .q42
"$CLI" ingest semantic "$STEM.nt"

echo ""
echo "=== built ==="
ls -lh "$STEM".{ttl,nt,q42} 2>/dev/null || true
echo "q42 vault: $STEM.q42"
