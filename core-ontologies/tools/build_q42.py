#!/usr/bin/env python3
"""
build_q42.py — compile the values-credentials corpus into native .q42 volumes.

For every `*.n3` under core-ontologies/ this drives the real engine pipeline
(`qualia-cli ingest semantic`) to produce a per-instrument `.q42` SuperBlock
volume in `core-ontologies/dist/q42/`, then *verifies the round-trip* by querying
each volume back (the SuperBlock reader + SPARQL BGP path). It writes a manifest
(`dist/q42/manifest.json`) mapping instrument → volume, triple count, and quins
read back.

Per-instrument volumes are produced deliberately (not one merged corpus volume):
the ingest parser hashes CURIE strings verbatim and does NOT expand `@prefix`, so
each instrument's `doc:` terms (`doc:Instrument`, `doc:article-1`, …) would COLLIDE
across files in a merged volume. Each instrument volume is internally consistent;
queries use the stored term forms (e.g. `?a a values:Undertaking`, not an expanded
IRI). See PLAN.md §3 for the prefix-expansion follow-up.

Usage:
    python tools/build_q42.py            # build + verify the whole corpus
    python tools/build_q42.py --no-verify
"""
from __future__ import annotations
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent          # core-ontologies/
REPO = ROOT.parent                                       # repo root
DIST = ROOT / "dist" / "q42"
VERIFY = "--no-verify" not in sys.argv


def find_cli() -> Path:
    # Pick the most recently built binary across profiles, so a stale release
    # build never shadows a freshly-rebuilt debug one (or vice versa).
    cands = [
        REPO / "target" / profile / name
        for profile in ("release", "debug")
        for name in ("qualia-cli.exe", "qualia-cli")
        if (REPO / "target" / profile / name).exists()
    ]
    if not cands:
        sys.exit("qualia-cli binary not found. Build it first: cargo build -p qualia-cli")
    return max(cands, key=lambda p: p.stat().st_mtime)


def corpus_files() -> list[Path]:
    # Every .n3 in the tree except generated dist artifacts.
    return sorted(p for p in ROOT.rglob("*.n3") if "dist" not in p.parts)


def run(cli: Path, *args: str) -> tuple[str, str]:
    """Run the CLI, decoding output as UTF-8 (the corpus text is UTF-8; Windows would
    otherwise default to cp1252 and yield None on undecodable bytes)."""
    p = subprocess.run(
        [str(cli), *args],
        capture_output=True, encoding="utf-8", errors="replace",
    )
    return (p.stdout or ""), (p.stderr or "")


def ingest(cli: Path, src: Path) -> tuple[Path, int]:
    """Copy `src` into DIST, ingest it to .q42, drop the .n3 copy. Returns (q42, triples)."""
    work_n3 = DIST / src.name
    shutil.copy2(src, work_n3)
    out, err = run(cli, "ingest", "semantic", str(work_n3))
    work_n3.unlink(missing_ok=True)
    q42 = work_n3.with_suffix(".q42")
    if "Done." not in out or not q42.exists():
        raise RuntimeError(f"ingest failed for {src.name}:\n{out}\n{err}")
    m = re.search(r"Triples:\s*(\d+)", out)
    return q42, int(m.group(1)) if m else 0


def verify(cli: Path, q42: Path) -> int:
    """Round-trip: read every triple back. Returns the count (>0 ⇒ readable). Selects only
    the subject so output stays small even though objects now resolve to full clause text."""
    out, err = run(cli, "query", "sparql", str(q42), "SELECT ?s WHERE { ?s ?p ?o }")
    m = re.search(r"(\d+)\s+result\(s\)", out)
    if not m:
        raise RuntimeError(f"verify query failed for {q42.name}:\n{out}\n{err}")
    return int(m.group(1))


def main() -> None:
    try:
        sys.stdout.reconfigure(encoding="utf-8")  # Windows consoles default to cp1252
    except Exception:
        pass
    cli = find_cli()
    DIST.mkdir(parents=True, exist_ok=True)
    files = corpus_files()
    print(f"Compiling {len(files)} ontology files -> {DIST.relative_to(REPO)}")
    print(f"Engine: {cli.relative_to(REPO)}   verify={VERIFY}\n")

    manifest, total_triples, total_quins, failures = [], 0, 0, []
    for src in files:
        rel = src.relative_to(ROOT).as_posix()
        try:
            q42, triples = ingest(cli, src)
            quins = verify(cli, q42) if VERIFY else -1
            total_triples += triples
            if quins >= 0:
                total_quins += quins
            manifest.append({
                "source": rel, "volume": q42.name,
                "triples": triples, "quins_read_back": quins,
            })
            flag = "" if (not VERIFY or quins > 0) else "  (!) 0 quins read back"
            print(f"  [OK] {rel:<68} {triples:>5} triples{flag}")
        except Exception as e:  # noqa: BLE001 — report and continue
            failures.append(rel)
            print(f"  [XX] {rel:<68} {e}")

    (DIST / "manifest.json").write_text(
        json.dumps({
            "instruments": len(manifest),
            "total_triples": total_triples,
            "total_quins_read_back": total_quins if VERIFY else None,
            "volumes": manifest,
        }, indent=2),
        encoding="utf-8",
    )

    print(f"\n{'='*60}")
    print(f"Volumes : {len(manifest)} / {len(files)}")
    print(f"Triples : {total_triples}")
    if VERIFY:
        print(f"Quins read back : {total_quins}")
    print(f"Manifest: {(DIST / 'manifest.json').relative_to(REPO)}")
    if failures:
        print(f"FAILURES ({len(failures)}): {', '.join(failures)}")
        sys.exit(1)
    print("Done.")


if __name__ == "__main__":
    main()
