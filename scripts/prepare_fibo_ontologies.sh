#!/usr/bin/env bash
# Merge EDMC FIBO RDF domains and ingest to .q42 volumes for the Ontology Demo.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RDF_ROOT="${FIBO_RDF_DIR:-$REPO_ROOT/bundled/ontologies/fibo/rdf}"
CATALOG="$REPO_ROOT/bundled/ontologies/fibo/catalog.json"
OUT_DIR="$REPO_ROOT/docs/data/fibo"
WORK_DIR="$REPO_ROOT/docs/data/fibo/.build"

mkdir -p "$OUT_DIR" "$WORK_DIR"

if [[ ! -f "$CATALOG" ]]; then
  echo "Missing catalog: $CATALOG" >&2
  exit 1
fi
if [[ ! -d "$RDF_ROOT" ]]; then
  echo "Missing FIBO RDF tree: $RDF_ROOT" >&2
  echo "Set FIBO_RDF_DIR or copy sources under bundled/ontologies/fibo/rdf/" >&2
  exit 1
fi

echo "FIBO ontology preparation"
echo "  RDF root   : $RDF_ROOT"
echo "  Output dir : $OUT_DIR"

python3 - "$CATALOG" "$RDF_ROOT" "$WORK_DIR" "$OUT_DIR" "$REPO_ROOT" <<'PY'
import json, subprocess, sys
from pathlib import Path

catalog_path, rdf_root, work_dir, out_dir, repo_root = map(Path, sys.argv[1:6])
catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
merge_py = repo_root / "scripts" / "merge_fibo_domain.py"
count = 0

for entry in catalog.get("ontologies", []):
    domain = entry["domain"]
    domain_dir = rdf_root / domain
    if not domain_dir.is_dir():
        print(f"  skip (missing domain): {domain}")
        continue
    base = domain.lower()
    nt_path = work_dir / f"{base}.nt"
    print(f"  merge: {domain}")
    subprocess.run(
        ["python3", str(merge_py), str(domain_dir), str(nt_path)],
        check=True,
    )
    print(f"  ingest: {base}.nt -> {base}.q42")
    subprocess.run(
        ["cargo", "run", "--release", "-p", "qualia-cli", "--", "ingest", "semantic", str(nt_path)],
        cwd=repo_root,
        check=True,
    )
    q42_src = nt_path.with_suffix(".q42")
    if not q42_src.is_file():
        raise SystemExit(f"ingest did not produce {q42_src}")
    dest = out_dir / f"{base}.q42"
    dest.write_bytes(q42_src.read_bytes())
    count += 1

print(f"  built {count} domain volumes")
PY

python3 "$REPO_ROOT/scripts/merge_ontology_manifest.py" fibo
echo "Sync complete — FIBO domains under $OUT_DIR/"