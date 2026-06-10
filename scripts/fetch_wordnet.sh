#!/usr/bin/env bash
# fetch_wordnet.sh
#
# Downloads Open English WordNet, ingests via qualia-cli, and installs a unified
# v2 `wordnet.q42` under Local_LIbraries/wordnet/ (override with QUALIA_WORDNET_DIR).
# Copies to docs/playground/ when --playground is passed (GitHub Pages CI).
#
# Requirements:
#   - curl or wget
#   - gzip (for decompression)
#   - cargo (to build qualia-cli)
#
# Usage:
#   bash scripts/fetch_wordnet.sh
#   bash scripts/fetch_wordnet.sh --subset 50000
#   bash scripts/fetch_wordnet.sh --subset 100000 --playground

set -euo pipefail

OEWN_VERSION="2025-edition"
OEWN_REPO="https://github.com/globalwordnet/english-wordnet"
OEWN_RELEASE_URL="${OEWN_REPO}/releases/download/${OEWN_VERSION}/english-wordnet-2025.ttl.gz"

WORK_DIR="$(pwd)/.wordnet_build"
RAW_GZ="${WORK_DIR}/english-wordnet-2025.ttl.gz"
RAW_NT="${WORK_DIR}/english-wordnet-2025.ttl"
OUTPUT_BASE="${WORK_DIR}/wordnet"
Q42_OUT="${OUTPUT_BASE}.q42"

PLAYGROUND_DIR="$(pwd)/docs/playground"
DATA_DIR="${QUALIA_WORDNET_DIR:-$(pwd)/Local_LIbraries/wordnet}"
SUBSET_LINES=""
COPY_PLAYGROUND=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --subset)
            SUBSET_LINES="$2"; shift 2 ;;
        --playground)
            COPY_PLAYGROUND=true; shift ;;
        *)
            echo "Unknown argument: $1" >&2; exit 1 ;;
    esac
done

mkdir -p "$DATA_DIR" "$WORK_DIR"

echo "=== Qualia-DB WordNet Ingestor (Q42 v2 unified volume) ==="
echo "  Release  : ${OEWN_VERSION}"
echo "  Work dir : ${WORK_DIR}"
echo "  Data dir : ${DATA_DIR}"
echo "  Output   : ${Q42_OUT}"
echo ""

if [[ ! -f "$RAW_GZ" ]]; then
    echo "[1/3] Downloading Open English WordNet ${OEWN_VERSION}..."
    if command -v curl &>/dev/null; then
        curl -L --progress-bar -o "$RAW_GZ" "$OEWN_RELEASE_URL"
    else
        wget -q --show-progress -O "$RAW_GZ" "$OEWN_RELEASE_URL"
    fi
else
    echo "[1/3] Skipping download — ${RAW_GZ} already exists."
fi

if [[ ! -f "$RAW_NT" ]]; then
    echo "[2/3] Decompressing..."
    gzip -dk "$RAW_GZ"
fi

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
fi

echo "[3/3] Building qualia-cli and ingesting unified v2 volume..."
cargo build --release -p qualia-cli --quiet

./target/release/qualia-cli ingest \
    --input  "$INGEST_INPUT" \
    --output "$OUTPUT_BASE"

if [[ ! -f "$Q42_OUT" ]]; then
    echo "ERROR: ${Q42_OUT} was not created." >&2
    exit 1
fi

Q42_BYTES=$(wc -c < "$Q42_OUT")
echo "      wordnet.q42 : $(( Q42_BYTES / 1024 / 1024 )) MB (lex + bidx + LZ4 blocks embedded)"

echo "Installing to ${DATA_DIR}/wordnet.q42 ..."
cp "$Q42_OUT" "${DATA_DIR}/wordnet.q42"

if [[ "$COPY_PLAYGROUND" == true ]]; then
    echo "Copying to docs/playground/ (Pages demo)..."
    mkdir -p "$PLAYGROUND_DIR"
    cp "$Q42_OUT" "${PLAYGROUND_DIR}/wordnet.q42"
fi

echo ""
echo "=== Done ==="
echo "  ${DATA_DIR}/wordnet.q42 — unified v2 volume ($(( Q42_BYTES / 1024 / 1024 )) MB)"
if [[ "$COPY_PLAYGROUND" == true ]]; then
echo "  ${PLAYGROUND_DIR}/wordnet.q42 — playground mirror"
fi
echo ""
echo "  Override install dir: QUALIA_WORDNET_DIR=/path bash scripts/fetch_wordnet.sh"
echo "  Lexicon is embedded — no .q42.lex / .q42.bidx sidecars required."
