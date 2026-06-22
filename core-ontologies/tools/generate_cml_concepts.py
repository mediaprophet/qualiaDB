#!/usr/bin/env python3
"""Lift the TEXT layer (un-instruments/*.n3) into the CML CONCEPT layer (concepts/*.n3).

For every typed provision in every instrument, emit:
  * a cml:Concept, cml:realizedBy the SAME provision node (joins via the one hash-space),
  * cml:asserts a deontic norm (values:Obligation/Prohibition/Right/Permission) with its
    bearer (borneBy/heldBy), cml:modality cml:Deontic, and — for duties borne by every
    Agent — the R1 universalisation a fortiori onto values:State,
  * meaningful METADATA context: provenance (prov:wasDerivedFrom the instrument source),
    dc:date, values:category, cml:modality, and curation status.

Everything is MACHINE-DERIVED and marked cml:Proposed + values:HeuristicDerived (the
corpus convention) — structurally inert / clearly provisional until a human attests
(cml:Attested / skos:exactMatch). This renders the whole human-rights corpus as one
queryable, modality-ready concept-graph so the logic engine can reason over every stated
obligation, right and prohibition; the hand-crafted concepts/ pilots are left untouched.

Run:  python3 core-ontologies/tools/generate_cml_concepts.py
"""
import re, os, glob

SRC = "core-ontologies/un-instruments"
OUT = "core-ontologies/concepts"

def _schema_version() -> str:
    """The canonical CML schema version (core-ontologies/SCHEMA_VERSION); stamped on every
    generated concept so staleness is detectable (see CML_UPGRADE.md)."""
    try:
        with open("core-ontologies/SCHEMA_VERSION", encoding="utf-8") as f:
            return f.read().strip() or "0"
    except OSError:
        return "0"

SCHEMA_VERSION = _schema_version()

# Hand-crafted, human-curated concept files — never overwrite or shadow.
KEEP = {"duty-to-suppress-forced-labour"}

NORMATIVE = {"values:Obligation", "values:Prohibition", "values:Right", "values:Permission"}


def esc(s):
    return re.sub(r"\s+", " ", s.replace("\\", "\\\\").replace('"', '\\"')).strip()


def field(block, pat):
    m = re.search(pat, block)
    return m.group(1) if m else None


def parse(text):
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    doc_ns = field(text, r"@prefix doc:\s+<([^>]+)>")
    title = date = source = category = None
    provisions = []
    for block in text.split("\n\n"):
        block = block.strip()
        if block.startswith("doc:Instrument"):
            title = field(block, r'dc:title "([^"]*)"')
            date = field(block, r'dc:date "([^"]*)"')
            source = field(block, r"values:source <([^>]+)>")
            category = field(block, r'values:category "([^"]*)"')
            continue
        m = re.match(r"doc:([a-z]+)-(\w+)\s+a\s+(values:\w+)\s*;", block)
        if not m:
            continue
        kind, n, ty = m.group(1), m.group(2), m.group(3)
        bm = re.search(r"values:(borneBy|heldBy)\s+values:(\w+)", block)
        bearer = (bm.group(1), bm.group(2)) if bm else None
        ptitle = field(block, r'dc:title "([^"]*)"') or f"{kind} {n}"
        provisions.append((kind, n, ty, bearer, ptitle))
    return doc_ns, title, date, source, category, provisions


def emit(slug, doc_ns, title, date, source, category, provisions):
    L = [
        "@prefix rdf:     <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .",
        "@prefix rdfs:    <http://www.w3.org/2000/01/rdf-schema#> .",
        "@prefix skos:    <http://www.w3.org/2004/02/skos/core#> .",
        "@prefix dc:      <http://purl.org/dc/terms/> .",
        "@prefix prov:    <http://www.w3.org/ns/prov#> .",
        "@prefix values:  <https://ns.webcivics.org/values/> .",
        "@prefix cml:     <https://ns.webcivics.org/cml/> .",
        "@prefix concept: <https://ns.webcivics.org/concept/> .",
        f"@prefix doc:     <{doc_ns}> .",
        "",
        "# ============================================================",
        f"# CML concept layer (LOGIC) — {title}",
        "# MACHINE-DERIVED from the TEXT layer (un-instruments) — cml:Proposed,",
        "# values:HeuristicDerived. Pending human attestation (cml:Attested / exactMatch).",
        "# One cml:Concept per provision; cml:realizedBy the provision; cml:asserts the",
        "# deontic norm. R1 universalisation materialised for duties borne by every Agent.",
        "# ============================================================",
        "",
    ]
    n_concepts = n_norms = 0
    for kind, n, ty, bearer, ptitle in provisions:
        cid = f"concept:{slug}-{kind}-{n}"
        nid = f"{cid}-norm"
        n_concepts += 1
        L.append(f"{cid} a cml:Concept ;")
        L.append(f'    skos:prefLabel "{esc(ptitle)}"@en ;')
        L.append(f"    cml:realizedBy doc:{kind}-{n} ;")
        L.append("    cml:curationStatus cml:Proposed ;")
        L.append(f'    cml:schemaVersion "{SCHEMA_VERSION}" ;   # regenerable — see CML_UPGRADE.md')
        if source:
            L.append(f"    prov:wasDerivedFrom <{source}> ;")
        if date:
            L.append(f'    dc:date "{esc(date)}" ;')
        if category:
            L.append(f'    values:category "{esc(category)}" ;')
        L.append(f"    cml:asserts {nid} .")
        L.append("")
        # The asserted deontic norm (the LOGIC).
        n_norms += 1
        L.append(f"{nid} a {ty} ;")
        L.append("    cml:modality cml:Deontic ;")
        L.append(f"    values:partOf {cid} ;")
        if bearer:
            L.append(f"    values:{bearer[0]} values:{bearer[1]} ;")
        if ty == "values:Obligation" and bearer and bearer[1] == "Agent":
            # R1 universalisation: a duty borne by every Agent is a fortiori borne by the State.
            L.append("    values:borneBy values:State ;   # R1 universalisation (Derived)")
        L.append("    values:deonticStatus values:HeuristicDerived ;")
        L.append("    cml:curationStatus cml:Proposed .")
        L.append("")
    return "\n".join(L), n_concepts, n_norms


def main():
    os.makedirs(OUT, exist_ok=True)
    files = tot_c = tot_n = 0
    for f in sorted(glob.glob(os.path.join(SRC, "*.n3"))):
        slug = os.path.basename(f)[:-3]
        if slug in KEEP:
            continue
        text = open(f, encoding="utf-8", errors="replace").read()
        doc_ns, title, date, source, category, provisions = parse(text)
        if not doc_ns or not provisions:
            continue
        out, n_c, n_n = emit(slug, doc_ns, title or slug, date, source, category, provisions)
        open(os.path.join(OUT, slug + ".n3"), "w", encoding="utf-8").write(out)
        files += 1
        tot_c += n_c
        tot_n += n_n
    print(f"Generated {files} CML concept files: {tot_c} concepts, {tot_n} deontic norms "
          f"(all cml:Proposed, pending attestation).")


if __name__ == "__main__":
    main()
