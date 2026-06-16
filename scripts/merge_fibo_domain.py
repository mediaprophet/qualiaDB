#!/usr/bin/env python3
"""Merge FIBO RDF/XML files for one domain into N-Triples (skips All* import catalogs)."""
import sys
from pathlib import Path

from rdflib import Graph


def merge_domain(domain_dir: Path, out_nt: Path) -> int:
    g = Graph()
    count = 0
    for rdf in sorted(domain_dir.rglob("*.rdf")):
        if rdf.name.startswith("All"):
            continue
        try:
            before = len(g)
            g.parse(rdf)
            added = len(g) - before
            count += 1
            print(f"  + {rdf.relative_to(domain_dir)} ({added} new triples)")
        except Exception as exc:
            print(f"  ! skip {rdf.name}: {exc}", file=sys.stderr)
    out_nt.parent.mkdir(parents=True, exist_ok=True)
    g.serialize(destination=out_nt, format="nt")
    print(f"  => {out_nt.name}: {len(g)} triples from {count} files")
    return len(g)


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit(f"usage: {sys.argv[0]} <domain_dir> <out.nt>")
    merge_domain(Path(sys.argv[1]), Path(sys.argv[2]))