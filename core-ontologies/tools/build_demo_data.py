#!/usr/bin/env python3
"""build_demo_data.py — distill the REAL values-credentials corpus into one compact JSON
that docs/values-credentials.html embeds. Nothing here is mocked: every count comes from
the actual .n3 corpus and the dist/q42/manifest.json round-trip.

For each instrument it records: title (@en), category, adoption date, triple count (from the
verified .q42 manifest), and the number of CML concepts + asserted deontic norms machine-
derived from it. It also extracts the non-derogable ICCPR Art. 4(2) list and a worked
TEXT->CONCEPT->LOGIC chain (UDHR Art. 1) so the page can show one real provision end-to-end.

Run:  python core-ontologies/tools/build_demo_data.py
Out:  docs/data/values-credentials.json
"""
from __future__ import annotations
import json, re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent      # core-ontologies/
REPO = ROOT.parent
UNI = ROOT / "un-instruments"
CON = ROOT / "concepts"
OUT = REPO / "docs" / "data" / "values-credentials.json"


def first(pat, text, default=None):
    m = re.search(pat, text)
    return m.group(1) if m else default


def lit(pat, text, default=None):
    """Match a quoted literal, tolerating an optional @lang / ^^datatype tag."""
    m = re.search(pat + r'\s+"((?:[^"\\]|\\.)*)"(?:@[\w-]+)?', text)
    return m.group(1) if m else default


def main():
    manifest = json.loads((ROOT / "dist" / "q42" / "manifest.json").read_text("utf-8"))
    triples_by_vol = {v["source"]: v["triples"] for v in manifest["volumes"]}

    instruments = []
    tot_c = tot_n = 0
    for f in sorted(UNI.glob("*.n3")):
        slug = f.stem
        text = f.read_text("utf-8", errors="replace")
        title = lit(r"dc:title", text, slug)
        category = lit(r"values:category", text, "instrument")
        date = lit(r"dc:date", text)
        cfile = CON / f"{slug}.n3"
        ctext = cfile.read_text("utf-8", errors="replace") if cfile.exists() else ""
        n_c = ctext.count("a cml:Concept")
        n_n = ctext.count("cml:asserts")
        tot_c += n_c
        tot_n += n_n
        instruments.append({
            "slug": slug,
            "title": title,
            "category": category,
            "date": date,
            "triples": triples_by_vol.get(f"un-instruments/{slug}.n3", 0),
            "concepts": n_c,
            "norms": n_n,
            "source": first(r"values:source <([^>]+)>", text),
        })

    instruments.sort(key=lambda i: i["norms"], reverse=True)

    # Non-derogable ICCPR Art. 4(2) overlay — verbatim transcription.
    overlay = (CON / "iccpr-non-derogable-overlay.n3").read_text("utf-8", errors="replace")
    nonderog = []
    for block in re.split(r"\n(?=concept:)", overlay):
        am = re.search(r"article-(\d+)-norm", block)
        if not am or "values:nonDerogable true" not in block:
            continue
        comment = first(r'rdfs:comment "([^"]+)"', block, "")
        nonderog.append({"article": int(am.group(1)), "label": comment.strip()})
    nonderog.sort(key=lambda x: x["article"])

    # Worked chain: UDHR Article 1 (TEXT -> CONCEPT -> LOGIC).
    udhr_t = (UNI / "udhr.n3").read_text("utf-8", errors="replace")
    udhr_c = (CON / "udhr.n3").read_text("utf-8", errors="replace")
    # first article block in the text layer
    art1_text = lit(r"dc:title", udhr_t.split("\n\n")[2] if len(udhr_t.split("\n\n")) > 2 else udhr_t)
    example = {
        "instrument": "Universal Declaration of Human Rights",
        "text_node": "doc:article-1",
        "text_label": "Article 1",
        "concept_node": first(r"(concept:udhr-\w+-1)\s+a\s+cml:Concept", udhr_c),
        "concept_label": lit(r"concept:udhr-\w+-1\s+a\s+cml:Concept\s*;\s*skos:prefLabel", udhr_c),
        "curation": "cml:Proposed",
        "provenance": "values:HeuristicDerived",
    }

    data = {
        "generated": "from core-ontologies/dist/q42/manifest.json + the .n3 corpus (no mock data)",
        "summary": {
            "instruments": len(instruments),
            "concepts": tot_c,
            "norms": tot_n,
            "volumes": manifest["instruments"],
            "triples": manifest["total_triples"],
            "quins_verified": manifest["total_quins_read_back"],
            "non_derogable": len(nonderog),
            "attested": 0,
        },
        "instruments": instruments,
        "nonDerogable": nonderog,
        "example": example,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"Wrote {OUT.relative_to(REPO)}: {len(instruments)} instruments, "
          f"{tot_c} concepts, {tot_n} norms, {len(nonderog)} non-derogable.")


if __name__ == "__main__":
    main()
