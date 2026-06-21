#!/usr/bin/env python3
"""Parse the authoritative ICRC IHL-treaty corpus -> values-credential N3.

Input: core-ontologies/_authoritative/icrc_corpus.txt — blocks delimited by
`@@@SLUG:<slug>@@@`, each with `TITLE:` / `DATE:` / `SOURCE:` metadata lines, then
provisions as a short header line ("Preamble" or "Article N - Title") followed by the
verbatim body (pulled from the ICRC Drupal JSON:API field_treaty_content).

These are the 1949 Geneva Conventions + 1977/2005 Additional Protocols — International
Humanitarian Law. Per Timothy's requirement, IHL binds in ANY armed conflict, declared
or not (Geneva Common Art 2 & 3); that context modelling lives in values.n3 / the SHACL
layer, not here. This generator produces the faithful base (verbatim originalText +
provenance), lexical-heuristic deontic typing (values:HeuristicDerived), category fixed
to IHL. Annex material (model agreements, medical forms) is preserved verbatim as one
`doc:annexes` provision rather than mis-segmented as normative undertakings.

Run:  python3 core-ontologies/tools/generate_icrc.py
"""
import re, os, sys

AUTH = "core-ontologies/_authoritative"
OUT  = "core-ontologies/un-instruments"
CORPUS = os.path.join(AUTH, "icrc_corpus.txt")
CATEGORY = "International Humanitarian Law (Law of War)"

def esc(s): return re.sub(r"\s+", " ", s.replace("\\","\\\\").replace('"','\\"')).strip()

def deontic(t):
    t = t.lower()
    if re.match(r"^\s*(no one|no party|no person|nothing|in no )\b", t) or "shall be prohibited" in t \
       or "shall not" in t or "are prohibited" in t or "is prohibited" in t or re.search(r"\bmay not be\b", t):
        return ("values:Prohibition", "values:borneBy values:Agent")
    if "has the right" in t or "have the right" in t or "entitled to" in t or "shall be entitled" in t:
        return ("values:Right", "values:heldBy values:NaturalPerson")
    if re.search(r"\bshall (ensure|take|undertake|guarantee|recognize|recognise|respect|protect|"
                 r"promote|adopt|prohibit|provide|treat|grant|afford|be bound)\b", t) \
       or "undertake to" in t or "shall be" in t or "shall enjoy" in t:
        return ("values:Obligation", "values:borneBy values:Agent")
    return ("values:Undertaking", "values:borneBy values:Agent")

def clean_header(h):
    """Collapse the doubled 'Article N - Article N - Title' the ICRC pre_title/title join
    produced, and the rare 'Preamble - Preamble'."""
    h = h.strip()
    m = re.match(r"^(Article\s+\d+\w*)\b", h, re.I)
    if m:
        n = m.group(1)
        # strip any number of leading repeats of "Article N -"
        rest = h
        while True:
            mm = re.match(r"^Article\s+\d+\w*\s*[-:]\s*", rest, re.I)
            if mm: rest = rest[mm.end():]
            else: break
        return n + (" - " + rest if rest and not re.match(r"^Article\s+\d+\w*$", h, re.I) else "")
    if re.match(r"^Preamble\b", h, re.I):
        return "Preamble"
    return h

def parse_block(slug, block):
    lines = block.split("\n")
    title = date = source = ""
    body_start = 0
    for i, ln in enumerate(lines[:6]):
        if ln.startswith("TITLE:"):  title  = ln[6:].strip()
        elif ln.startswith("DATE:"): date   = ln[5:].strip()
        elif ln.startswith("SOURCE:"): source = ln[7:].strip(); body_start = i + 1
    lines = lines[body_start:]

    art_hdr = re.compile(r"^Article\s+(\d+)\w*\b", re.I)
    struct  = re.compile(r"^(Chapter|Part|Section|Title|Final provisions)\b", re.I)
    preamble = ""
    segs = []            # (sortkey, label, id, body)
    annex_buf = []
    in_annex = False
    seen_max = 0         # highest main-article number seen (annexes restart numbering)
    cur = None           # ('preamble'|('art',n)|label, [body lines])

    def flush():
        if not cur: return
        kind, n, label, buf = cur
        text = "\n".join(buf).strip()
        if not text: return
        if kind == "pre":
            nonlocal preamble; preamble = text
        elif kind == "art":
            segs.append((n, label, f"article-{n}", text))

    prevblank = True
    for ln in lines:
        s = ln.strip()
        is_short = len(s) < 170
        am = art_hdr.match(s)
        # Annex boundary, format-independent. Conventions label annexes inconsistently
        # ("Annex N :", "ANNEX I.", or with no "Annex" word at all) but annex articles
        # ALWAYS restart numbering, while the main body increases monotonically. So the
        # boundary is: (a) a SHORT header line starting with "Annex"/"ANNEX", or
        # (b) an "Article N" header whose number does not advance past the running max
        # once we are well into the body (reset = annex restart / duplicate run).
        if not in_annex and is_short and prevblank:
            if re.match(r"^Annex\b", s, re.I):
                flush(); cur = None; in_annex = True
            elif am and seen_max >= 5 and int(am.group(1)) <= seen_max:
                flush(); cur = None; in_annex = True
        if in_annex:
            if s: annex_buf.append(ln)
            prevblank = (s == "")
            continue
        if s.lower().startswith("preamble") and is_short and prevblank:
            flush(); cur = ("pre", 0, "Preamble", [])
        elif am and is_short and prevblank:
            flush(); n = int(am.group(1)); seen_max = max(seen_max, n); cur = ("art", n, clean_header(s), [])
        elif struct.match(s) and is_short and prevblank:
            flush(); cur = None          # structural divider: skip, no body of its own
        else:
            if cur: cur[3].append(ln)
        prevblank = (s == "")
    flush()
    annex = "\n".join(annex_buf).strip()
    return title, date, source, preamble, segs, annex

def emit(slug, title, date, source, preamble, segs, annex):
    ns = "https://ns.webcivics.org/values/" + slug + "#"
    L = ['@prefix rdf:    <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .',
         '@prefix rdfs:   <http://www.w3.org/2000/01/rdf-schema#> .',
         '@prefix dc:     <http://purl.org/dc/terms/> .',
         '@prefix values: <https://ns.webcivics.org/values/> .',
         f'@prefix doc:    <{ns}> .', '',
         '# ============================================================',
         f'# {title}',
         '# RE-SOURCED from authoritative ICRC IHL database (verbatim originalText).',
         '# International Humanitarian Law: binds in ANY armed conflict, declared or not',
         '# (Geneva Common Art 2 & 3). Deontic typing = lexical heuristic',
         '# (values:HeuristicDerived) — pending review. Annexes preserved verbatim.',
         '# ============================================================', '',
         'doc:Instrument a values:ValuesCredential ;',
         f'    dc:title "{esc(title)}" ;']
    if re.search(r"\d{4}", date): L.append(f'    dc:date "{esc(date)}" ;')
    L.append(f'    values:category "{esc(CATEGORY)}" ;')
    L.append('    values:categoryStatus values:AutoAssigned ;')
    L.append(f'    values:source <{source}> ;')
    L.append('    rdfs:comment "Re-sourced from authoritative ICRC text; deontic + amendment layers pending." .')
    L.append('')
    if len(preamble) > 40:
        L += ['doc:preamble a values:Undertaking ; values:partOf doc:Instrument ;',
              '    values:borneBy values:Agent ;',
              '    values:deonticStatus values:HeuristicDerived ;',
              f'    values:originalText "{esc(preamble)}" .', '']
    for n, label, pid, body in sorted(segs):
        if len(body) < 8: continue
        ty, bearer = deontic(body)
        L.append(f'doc:{pid} a {ty} ;')
        L.append('    values:partOf doc:Instrument ;')
        L.append(f'    {bearer} ;')
        L.append(f'    dc:title "{esc(label)}" ;')
        L.append('    values:deonticStatus values:HeuristicDerived ;')
        L.append(f'    values:originalText "{esc(body)}" .')
        L.append('')
    if len(annex) > 40:
        L += ['doc:annexes a values:Undertaking ; values:partOf doc:Instrument ;',
              '    values:borneBy values:Agent ;',
              '    dc:title "Annexes" ;',
              '    rdfs:comment "Annex material (model agreements / forms) preserved verbatim; not segmented as normative undertakings." ;',
              '    values:deonticStatus values:HeuristicDerived ;',
              f'    values:originalText "{esc(annex)}" .', '']
    return "\n".join(L)

def main():
    if not os.path.exists(CORPUS):
        sys.exit("Corpus not found: " + CORPUS)
    os.makedirs(OUT, exist_ok=True)
    blob = open(CORPUS, encoding="utf-8", errors="replace").read().replace("\r", "")
    parts = re.split(r"@@@SLUG:(.*?)@@@\n", blob)
    report = []
    for i in range(1, len(parts), 2):
        slug = parts[i].strip()
        title, date, source, pre, segs, annex = parse_block(slug, parts[i+1])
        out = os.path.join(OUT, slug + ".n3")
        open(out, "w", encoding="utf-8").write(emit(slug, title, date, source, pre, segs, annex))
        nums = sorted(n for n,_,_,_ in segs)
        # gap check (1..max sequential)
        gaps = [k for k in range(1, (nums[-1] if nums else 0)+1) if k not in nums] if nums else []
        report.append((slug, len(segs), nums[-1] if nums else 0, gaps, bool(pre), bool(annex)))
    print("slug            provs  maxArt  preamble annex  gaps")
    for slug, n, mx, gaps, pre, annex in report:
        g = "none" if not gaps else (str(gaps[:8]) + ("..." if len(gaps) > 8 else ""))
        print(f"  {slug:14s} {n:4d}   {mx:4d}    {str(pre):5s}   {str(annex):5s}  {g}")

if __name__ == "__main__":
    main()
