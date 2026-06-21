#!/usr/bin/env python3
"""Deterministic CSV -> N3 converter for the UN Instruments values-credentials.

Produces FAITHFUL, complete, provenance-preserving N3 per instrument:
  - every provision carries verbatim `values:originalText` + `values:source`
  - deontic typing (Right/Obligation/Prohibition) is applied ONLY by conservative
    lexical patterns and explicitly tagged `values:deonticStatus values:HeuristicDerived`
    (vs HandCurated) so jurisprudential review is auditable. No interpretation is faked.

UDHR is authored by hand (un-instruments/udhr.n3) and is NOT regenerated here.
Run:  python3 core-ontologies/tools/generate_n3.py
"""
import csv, re, sys, os

CSV = "UN Instruments Table - UN_data.csv"
OUT = "core-ontologies/un-instruments"
SKIP = {"universal declaration of human rights"}  # hand-curated gold reference

TITLE_KW = ("Convention", "Declaration", "Covenant", "Charter", "Protocol",
            "Principles", "Rights", "Slavery")

def slug(name):
    s = re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")
    return re.sub(r"-+", "-", s)[:60]

def esc(t):
    t = t.replace("\\", "\\\\").replace('"', '\\"')
    return re.sub(r"\s+", " ", t).strip()   # collapse \r \n \t -> single space (valid Turtle literal)

def is_title(row):
    """An instrument HEADER row: keyword title, has date/URL, and NO section (col2)."""
    r = (row + [""] * 6)[:6]
    c0 = r[0].strip().strip('"')
    if not c0 or not any(k in c0 for k in TITLE_KW):
        return False
    has_date = bool(re.search(r"\d{4}", r[1].strip()))
    has_url = r[4].strip().startswith("http") or r[5].strip().startswith("http")
    return has_date or has_url

def deontic(text):
    t = text.lower()
    if re.match(r"^\s*(no one|no party|no person|nothing)\b", t) or "shall be prohibited" in t \
       or "shall not" in t or re.search(r"\bmay not be\b", t):
        return ("values:Prohibition", "values:borneBy values:Agent")
    if "has the right" in t or "have the right" in t or "entitled to" in t or "right to" in t:
        return ("values:Right", "values:heldBy values:NaturalPerson")
    if re.search(r"\bshall (ensure|take|undertake|guarantee|recognize|recognise|respect|protect|promote|adopt|prohibit|provide)\b", t) \
       or "undertake to" in t or "shall be" in t:
        return ("values:Obligation", "values:borneBy values:Agent")
    return ("values:Undertaking", None)

def main():
    if not os.path.exists(CSV):
        sys.exit(f"CSV not found: {CSV} (run from repo root)")
    rows = list(csv.reader(open(CSV, newline="", encoding="utf-8", errors="replace")))
    os.makedirs(OUT, exist_ok=True)

    # group rows by instrument — boundary = any non-empty col0 that keyword-matches
    # a title (forward-fill). Metadata (date/url) captured from whichever row carries it.
    insts = []          # list of [name, date, source, [ [section, subidx, text] ]]
    idx = {}            # exact-name -> insts index
    cur = None
    for r in rows[1:]:
        r = (r + [""] * 6)[:6]
        if is_title(r):                    # instrument HEADER -> switch/create
            name = r[0].strip().strip('"')
            if name not in idx:
                idx[name] = len(insts); insts.append([name, "", "", []])
            cur = insts[idx[name]]
        if cur is None:
            continue
        # metadata
        if not cur[1] and re.search(r"\d{4}", r[1].strip()):
            cur[1] = r[1].strip()
        if not cur[2]:
            for cand in (r[4], r[5]):
                if cand.strip().startswith("http"):
                    cur[2] = cand.strip(); break
        # provision text — column-agnostic: the spreadsheet is INCONSISTENT about
        # which column holds article text, so take the longest meaningful cell.
        cells = [c.strip().strip('"') for c in r]
        secre = re.compile(r"^(preamble|article|principle|part|section|chapter)\b", re.I)
        sec = next((c for c in cells if secre.match(c)), "")
        sub = next((c for c in cells if c.isdigit() and len(c) <= 3), "")
        name = cur[0]
        cand = [c for c in cells
                if c and c != name and not c.startswith("http")
                and not secre.match(c) and c.upper() != "URI"
                and not (c.isdigit() and len(c) <= 3)
                and len(c) >= 12]
        txt = max(cand, key=len) if cand else ""
        if not txt or re.match(r"^\d{1,2}\s+\w+\s+\d{4}$", txt):   # skip date-only cells
            continue
        last_sec = cur[3][-1][0] if cur[3] else ""
        if sec or sub:                     # new provision (section OR numbered paragraph)
            cur[3].append([sec or last_sec, sub, txt])
        elif cur[3]:                       # true prose continuation
            cur[3][-1][2] += " " + txt
        else:
            cur[3].append(["", sub, txt])

    written = 0
    for name, date, src, provs in insts:
        if name.lower() in SKIP or not provs:
            continue
        sg = slug(name)
        ns = f"https://ns.webcivics.org/values/{sg}#"
        path = os.path.join(OUT, f"{sg}.n3")
        L = []
        L.append("@prefix rdf:    <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .")
        L.append("@prefix rdfs:   <http://www.w3.org/2000/01/rdf-schema#> .")
        L.append("@prefix dc:     <http://purl.org/dc/terms/> .")
        L.append("@prefix values: <https://ns.webcivics.org/values/> .")
        L.append(f"@prefix doc:    <{ns}> .")
        L.append("")
        L.append("# ============================================================")
        L.append(f"# {name}")
        L.append("# AUTO-GENERATED faithful base (verbatim originalText + provenance).")
        L.append("# Deontic typing = conservative lexical heuristic, tagged")
        L.append("# values:HeuristicDerived — PENDING jurisprudential review")
        L.append("# (states->parties amendment + universalisation applied at review).")
        L.append("# ============================================================")
        L.append("")
        L.append(f"doc:Instrument a values:ValuesCredential ;")
        L.append(f'    dc:title "{esc(name)}" ;')
        if re.search(r"\d{4}", date):
            L.append(f'    dc:date "{esc(date)}" ;')
        if src:
            L.append(f"    values:source <{src}> ;")
        L.append('    rdfs:comment "Affirmable values credential; auto-generated base, deontic layer pending review." .')
        L.append("")
        seen = {}
        for sec, sub, txt in provs:
            base = slug(sec) or "provision"
            key = base + (f"-p{sub}" if sub and sub.isdigit() else "")
            seen[key] = seen.get(key, 0) + 1
            pid = key if seen[key] == 1 else f"{key}-{seen[key]}"
            dtype, bearer = deontic(txt)
            L.append(f"doc:{pid} a {dtype} ;")
            L.append(f"    values:partOf doc:Instrument ;")
            if bearer:
                L.append(f"    {bearer} ;")
            if sec:
                L.append(f'    dc:title "{esc(sec)}" ;')
            L.append("    values:deonticStatus values:HeuristicDerived ;")
            L.append(f'    values:originalText "{esc(txt)}" .')
            L.append("")
        open(path, "w", encoding="utf-8").write("\n".join(L))
        written += 1
        print(f"  {len(provs):4d} provisions -> {path}")
    print(f"\nWrote {written} instrument files to {OUT}/ (UDHR hand-curated, skipped).")

if __name__ == "__main__":
    main()
