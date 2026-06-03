#!/usr/bin/env bash
# fetch_wordnet.sh
#
# Downloads the Open English WordNet 2024 N-Triples release, ingests it into
# the Qualia-DB binary format, and copies the resulting artefacts to the
# playground directory for the GitHub Pages demo.
#
# Requirements:
#   - curl or wget
#   - gzip (for decompression)
#   - cargo (to build qualia-cli)
#
# Usage:
#   bash scripts/fetch_wordnet.sh
#   bash scripts/fetch_wordnet.sh --subset 50000   # limit to first N triples

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration — pin the exact release for reproducibility
# ---------------------------------------------------------------------------

OEWN_VERSION="2025-edition"
OEWN_REPO="https://github.com/globalwordnet/english-wordnet"
# 2025 edition ships Turtle (.ttl.gz); the .nt.gz format was dropped after 2023.
# qualia-cli ingest treats .ttl as RDF (rio_turtle parser) — see ingest.rs is_rdf check.
OEWN_RELEASE_URL="${OEWN_REPO}/releases/download/${OEWN_VERSION}/english-wordnet-2025.ttl.gz"
OEWN_SHA256="UNVERIFIED"

WORK_DIR="$(pwd)/.wordnet_build"
RAW_GZ="${WORK_DIR}/english-wordnet-2025.ttl.gz"
RAW_NT="${WORK_DIR}/english-wordnet-2025.ttl"
OUTPUT_BASE="${WORK_DIR}/wordnet"
Q42_OUT="${OUTPUT_BASE}.q42"
LEX_OUT="${OUTPUT_BASE}.q42.lex"
BIDX_OUT="${OUTPUT_BASE}.q42.bidx"
CQ42_OUT="${OUTPUT_BASE}.c.q42"
LEX_LZ4_OUT="${OUTPUT_BASE}.q42.lex.lz4"

PLAYGROUND_DIR="$(pwd)/docs/playground"
SUBSET_LINES=""

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------

while [[ $# -gt 0 ]]; do
    case "$1" in
        --subset)
            SUBSET_LINES="$2"; shift 2 ;;
        *)
            echo "Unknown argument: $1" >&2; exit 1 ;;
    esac
done

# ---------------------------------------------------------------------------
# Preparation
# ---------------------------------------------------------------------------

mkdir -p "$WORK_DIR"

echo "=== Qualia-DB WordNet Ingestor ==="
echo "  Release  : ${OEWN_VERSION}"
echo "  Work dir : ${WORK_DIR}"
echo "  Output   : ${Q42_OUT}"
echo ""

# ---------------------------------------------------------------------------
# 1. Download (skip if already present)
# ---------------------------------------------------------------------------

if [[ ! -f "$RAW_GZ" ]]; then
    echo "[1/4] Downloading Open English WordNet ${OEWN_VERSION}..."
    if command -v curl &>/dev/null; then
        curl -L --progress-bar -o "$RAW_GZ" "$OEWN_RELEASE_URL"
    else
        wget -q --show-progress -O "$RAW_GZ" "$OEWN_RELEASE_URL"
    fi
else
    echo "[1/4] Skipping download — ${RAW_GZ} already exists."
fi

# ---------------------------------------------------------------------------
# 2. Decompress
# ---------------------------------------------------------------------------

if [[ ! -f "$RAW_NT" ]]; then
    echo "[2/4] Decompressing..."
    gzip -dk "$RAW_GZ"
fi

# Optional subset (useful for fast iteration / CI)
INGEST_INPUT="$RAW_NT"
if [[ -n "$SUBSET_LINES" ]]; then
    SUBSET_FILE="${WORK_DIR}/wordnet_subset_${SUBSET_LINES}.nt"
    if [[ ! -f "$SUBSET_FILE" ]]; then
        echo "      Creating subset: first ${SUBSET_LINES} non-comment triples..."
        grep -v '^#' "$RAW_NT" | head -n "$SUBSET_LINES" > "$SUBSET_FILE"
    fi
    INGEST_INPUT="$SUBSET_FILE"
    OUTPUT_BASE="${WORK_DIR}/wordnet_subset_${SUBSET_LINES}"
    Q42_OUT="${OUTPUT_BASE}.q42"
    LEX_OUT="${OUTPUT_BASE}.q42.lex"
    echo "      Subset input: ${INGEST_INPUT}"
fi

# ---------------------------------------------------------------------------
# 3. Build qualia-cli and ingest
# ---------------------------------------------------------------------------

echo "[3/4] Building qualia-cli (release)..."
cargo build --release -p qualia-cli --quiet

echo "      Running ingestor..."
./target/release/qualia-cli ingest \
    --input  "$INGEST_INPUT" \
    --output "$OUTPUT_BASE"

echo "      Verifying output files..."
if [[ ! -f "$Q42_OUT" ]]; then
    echo "ERROR: ${Q42_OUT} was not created." >&2; exit 1
fi
if [[ ! -f "$LEX_OUT" ]]; then
    echo "ERROR: ${LEX_OUT} was not created." >&2; exit 1
fi
if [[ ! -f "$BIDX_OUT" ]]; then
    echo "ERROR: ${BIDX_OUT} was not created." >&2; exit 1
fi

Q42_BYTES=$(wc -c < "$Q42_OUT")
LEX_BYTES=$(wc -c < "$LEX_OUT")
BIDX_BYTES=$(wc -c < "$BIDX_OUT")
echo "      wordnet.q42      : $(( Q42_BYTES / 1024 / 1024 )) MB"
echo "      wordnet.q42.lex  : $(( LEX_BYTES / 1024 )) KB"
echo "      wordnet.q42.bidx : $(( BIDX_BYTES / 1024 )) KB  ($(( BIDX_BYTES / 16 - 1 )) block ranges)"

# ---------------------------------------------------------------------------
# 4. Compress artefacts (LZ4 block-stream, strip SuperBlock headers)
# ---------------------------------------------------------------------------

echo "[4/5] Compressing artefacts with qualia-cli..."
./target/release/qualia-cli compress --input "$Q42_OUT"  --output "$CQ42_OUT"
./target/release/qualia-cli compress --input "$LEX_OUT"  --output "$LEX_LZ4_OUT"

CQ42_BYTES=$(wc -c < "$CQ42_OUT")
LEX_LZ4_BYTES=$(wc -c < "$LEX_LZ4_OUT")
echo "      wordnet.c.q42        : $(( CQ42_BYTES / 1024 / 1024 )) MB  ($(( CQ42_BYTES * 100 / Q42_BYTES ))% of original)"
echo "      wordnet.q42.lex.lz4  : $(( LEX_LZ4_BYTES / 1024 )) KB"

# ---------------------------------------------------------------------------
# 5. Copy artefacts to playground
# ---------------------------------------------------------------------------

echo "[5/5] Copying artefacts to playground/..."
cp "$Q42_OUT"      "${PLAYGROUND_DIR}/wordnet.q42"
cp "$LEX_OUT"      "${PLAYGROUND_DIR}/wordnet.q42.lex"
cp "$BIDX_OUT"     "${PLAYGROUND_DIR}/wordnet.q42.bidx"
cp "$CQ42_OUT"     "${PLAYGROUND_DIR}/wordnet.c.q42"
cp "$LEX_LZ4_OUT"  "${PLAYGROUND_DIR}/wordnet.q42.lex.lz4"

echo ""
echo "=== Done ==="
echo ""
echo "  Demand-paging (default, HTTP Range requests + BIDX):"
echo "    playground/wordnet.q42         — $(( Q42_BYTES / 1024 / 1024 )) MB  (object-sorted SuperBlocks)"
echo "    playground/wordnet.q42.lex     — $(( LEX_BYTES / 1024 )) KB  (binary lexicon)"
echo "    playground/wordnet.q42.bidx    — $(( BIDX_BYTES / 1024 )) KB  (block-range index → 1-3 req/query)"
echo ""
echo "  Bulk-load fallback (compressed LZ4 stream, flat Quins):"
echo "    playground/wordnet.c.q42       — $(( CQ42_BYTES / 1024 / 1024 )) MB"
echo "    playground/wordnet.q42.lex.lz4 — $(( LEX_LZ4_BYTES / 1024 )) KB"
echo ""
echo "  vfs-manifest.json is set to wordnet.q42 (demand-paging)."
echo "  Switch 'url' to wordnet.c.q42 + 'compressed: true' for bulk-load mode."
echo ""
echo "Next step: rebuild the WASM module so execute_ntriples_query is exported:"
echo "  wasm-pack build crates/qualia-core-db --target web --out-dir ../../playground --no-typescript"
echo ""
echo "Then commit playground/ artefacts for the GitHub Pages deploy."
