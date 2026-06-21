#!/usr/bin/env python3
"""Rebuild core-ontologies/INDEX.md by scanning ALL values-credential .n3 files
(un-instruments/ = OHCHR + ICRC, regional/ = Commonwealth etc.). Source-agnostic:
reads dc:title, values:category, dc:date and counts provisions per instrument, then
groups by category. Run after any generator: python3 core-ontologies/tools/build_index.py
"""
import os, glob, rdflib

ROOT = "core-ontologies"
DIRS = ["un-instruments", "regional"]
V  = rdflib.Namespace("https://ns.webcivics.org/values/")
DC = rdflib.Namespace("http://purl.org/dc/terms/")

def main():
    rows = []  # (category, title, date, nprov, relpath)
    files = []
    for d in DIRS:
        files += sorted(glob.glob(os.path.join(ROOT, d, "*.n3")))
    for f in files:
        g = rdflib.Graph()
        try: g.parse(f, format="turtle")
        except Exception as e:
            print(f"  !! parse error {f}: {e}"); continue
        cred = next(g.subjects(rdflib.RDF.type, V.ValuesCredential), None)
        if cred is None: continue
        title = str(next(g.objects(cred, DC.title), os.path.basename(f)))
        date  = str(next(g.objects(cred, DC.date), ""))
        cat   = str(next(g.objects(cred, V.category), "Uncategorised"))
        nprov = len(set(g.subjects(V.partOf, None)))
        rel   = os.path.relpath(f, ROOT).replace("\\", "/")
        rows.append((cat, title, date, nprov, rel))

    cats = {}
    for cat, title, date, n, rel in rows:
        cats.setdefault(cat, []).append((title, date, n, rel))

    L = ["# Values Credentials — Categorised Index", "",
         f"**{len(rows)} instruments** re-expressed as affirmable values-credentials "
         "(`<slug>.n3`, verbatim `values:originalText` + provenance). Sources: OHCHR "
         "(UN human-rights instruments), ICRC (Geneva Conventions & Additional Protocols), "
         "and regional charters. Categories and deontic typing are auto/heuristic "
         "(`values:categoryStatus AutoAssigned`, `values:deonticStatus HeuristicDerived`) "
         "— jurisprudential review pending.", ""]
    for cat in sorted(cats):
        items = sorted(cats[cat])
        L.append(f"## {cat}  ({len(items)})")
        L.append("")
        for title, date, n, rel in items:
            d = f" — {date}" if date.strip() else ""
            L.append(f"- **{title}**{d} · {n} provisions · `{rel}`")
        L.append("")
    open(os.path.join(ROOT, "INDEX.md"), "w", encoding="utf-8").write("\n".join(L))
    print(f"Wrote {ROOT}/INDEX.md — {len(rows)} instruments, {len(cats)} categories.")

if __name__ == "__main__":
    main()
