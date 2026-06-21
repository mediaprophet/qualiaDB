#!/usr/bin/env python3
"""Rebuild core-ontologies/INDEX.md by scanning ALL values-credential .n3 files
(un-instruments/ = OHCHR + ICRC, regional/ = Commonwealth etc.). Source-agnostic:
reads dc:title, values:category, dc:date and counts provisions per instrument,
groups by category, AND emits a governance gap-report dashboard (PLAN §9.1):
field coverage, tier/legalForm/bindingStatus distributions, scraper-boilerplate
flags, one-provision soft-law outliers, parse errors.

This is a REPORT (always exits 0). The failing CI gate is the native Rust
`validate_core_ontologies`. Run after any generator:
    python3 core-ontologies/tools/build_index.py
"""
import os, glob, rdflib

ROOT = "core-ontologies"
DIRS = ["un-instruments", "regional", "mutable"]
V  = rdflib.Namespace("https://ns.webcivics.org/values/")
DC = rdflib.Namespace("http://purl.org/dc/terms/")

# Governance fields on the CREDENTIAL node (predicate local-name on V).
CRED_FIELDS = [
    "tier", "legalForm", "bindingStatus", "integrityHash",
    "source", "curationStatus", "category", "categoryStatus",
]
# Provision-level field: present iff ANY provision (values:partOf cred) carries it.
PROV_FIELDS = ["deonticStatus", "originalText"]
GOV_FIELDS = CRED_FIELDS + PROV_FIELDS
# Scraper boilerplate markers that must not survive into values:originalText.
BOILERPLATE = ["Download: PDF", "Download:", "Print this page", "Share via"]


def one(g, s, p, default=""):
    return str(next(g.objects(s, p), default))


def main():
    files = []
    for d in DIRS:
        files += sorted(glob.glob(os.path.join(ROOT, d, "*.n3")))

    insts = []          # list of per-instrument dicts
    parse_errors = []   # (relpath, message)
    for f in files:
        rel = os.path.relpath(f, ROOT).replace("\\", "/")
        g = rdflib.Graph()
        try:
            g.parse(f, format="turtle")
        except Exception as e:
            parse_errors.append((rel, str(e)))
            continue
        cred = next(g.subjects(rdflib.RDF.type, V.ValuesCredential), None)
        if cred is None:
            continue
        rec = {
            "rel": rel,
            "title": one(g, cred, DC.title, os.path.basename(f)),
            "date": one(g, cred, DC.date),
            "category": one(g, cred, V.category, "Uncategorised"),
            "nprov": len(set(g.subjects(V.partOf, None))),
        }
        for fld in CRED_FIELDS:
            rec[fld] = one(g, cred, V[fld]) if (cred, V[fld], None) in g else ""
        # Provision-level: present iff any provision carries the predicate.
        provisions = list(g.subjects(V.partOf, cred))
        for fld in PROV_FIELDS:
            rec[fld] = "present" if any((p, V[fld], None) in g for p in provisions) else ""
        # Scraper boilerplate: scan ALL originalText literals (provision-level).
        all_otext = [str(o) for o in g.objects(None, V.originalText)]
        rec["boilerplate"] = sorted({m for t in all_otext for m in BOILERPLATE if m in t})
        insts.append(rec)

    # ── Categorised index (unchanged behaviour) ────────────────────────────────
    cats = {}
    for r in insts:
        cats.setdefault(r["category"], []).append(r)

    L = ["# Values Credentials — Categorised Index", "",
         f"**{len(insts)} instruments** re-expressed as affirmable values-credentials "
         "(`<slug>.n3`, verbatim `values:originalText` + provenance). Sources: OHCHR "
         "(UN human-rights instruments), ICRC (Geneva Conventions & Additional Protocols), "
         "and regional charters. Categories and deontic typing are auto/heuristic "
         "(`values:categoryStatus AutoAssigned`, `values:deonticStatus HeuristicDerived`) "
         "— jurisprudential review pending.", ""]
    for cat in sorted(cats):
        items = sorted(cats[cat], key=lambda r: (r["title"], r["date"]))
        L.append(f"## {cat}  ({len(items)})")
        L.append("")
        for r in items:
            d = f" — {r['date']}" if r["date"].strip() else ""
            L.append(f"- **{r['title']}**{d} · {r['nprov']} provisions · `{r['rel']}`")
        L.append("")

    # ── Governance gap report (PLAN §9.1) ──────────────────────────────────────
    n = len(insts)
    coverage = {fld: sum(1 for r in insts if r[fld].strip()) for fld in GOV_FIELDS}

    def dist(fld):
        out = {}
        for r in insts:
            out[r[fld] or "(missing)"] = out.get(r[fld] or "(missing)", 0) + 1
        return out

    boilerplate_hits = [(r["rel"], r["boilerplate"]) for r in insts if r["boilerplate"]]
    one_prov = [r["rel"] for r in insts if r["nprov"] <= 1]

    G = ["", "---", "", "## Governance Gap Report  (generated — PLAN §9.1)", "",
         f"Scanned **{n} instruments**" +
         (f", **{len(parse_errors)} parse errors**." if parse_errors else "."), "",
         "### Field coverage (present / total)"]
    for fld in GOV_FIELDS:
        miss = n - coverage[fld]
        flag = "  ⚠ **all missing**" if coverage[fld] == 0 else (f"  ⚠ {miss} missing" if miss else "")
        G.append(f"- `values:{fld}` — {coverage[fld]}/{n}{flag}")
    G += ["", "### Distribution"]
    for fld in ("tier", "legalForm", "bindingStatus", "curationStatus", "deonticStatus"):
        G.append(f"- **{fld}**: " + ", ".join(f"{k} ({v})" for k, v in sorted(dist(fld).items())))
    G += ["", "### Flags"]
    G.append(f"- **Scraper boilerplate** ({len(boilerplate_hits)} files) — strip from `values:originalText`:")
    for rel, markers in boilerplate_hits[:40]:
        G.append(f"  - `{rel}` — {', '.join(markers)}")
    if len(boilerplate_hits) > 40:
        G.append(f"  - … and {len(boilerplate_hits) - 40} more")
    G.append(f"- **One-provision (soft-law) outliers** ({len(one_prov)}): " +
             (", ".join(f"`{p}`" for p in one_prov[:20]) + (" …" if len(one_prov) > 20 else "") if one_prov else "none"))
    if parse_errors:
        G.append(f"- **Parse errors** ({len(parse_errors)}):")
        for rel, msg in parse_errors[:20]:
            G.append(f"  - `{rel}` — {msg[:120]}")

    open(os.path.join(ROOT, "INDEX.md"), "w", encoding="utf-8").write("\n".join(L + G))

    # ── Console summary ─────────────────────────────────────────────────────────
    print(f"Wrote {ROOT}/INDEX.md — {n} instruments, {len(cats)} categories.")
    print("Governance gap report:")
    for fld in GOV_FIELDS:
        miss = n - coverage[fld]
        if miss:
            print(f"  - {fld}: {coverage[fld]}/{n} present  ({miss} MISSING)")
    if boilerplate_hits:
        print(f"  - scraper boilerplate in {len(boilerplate_hits)} files (strip from originalText)")
    if one_prov:
        print(f"  - one-provision soft-law outliers: {len(one_prov)}")
    if parse_errors:
        print(f"  - PARSE ERRORS: {len(parse_errors)}")


if __name__ == "__main__":
    main()
