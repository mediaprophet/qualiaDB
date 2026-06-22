#!/usr/bin/env python3
"""Parse authoritative OHCHR instrument text (get_page_text dumps) -> values-credential N3.

Input: either individual `core-ontologies/_authoritative/ohchr__<slug>.txt` files, or a
combined `ohchr_corpus.txt` with `@@@SLUG:<slug>@@@` delimiters. Output: per-instrument N3
in core-ontologies/un-instruments/ with correct Article/Principle structure + completeness,
verbatim values:originalText, lexical-heuristic deontic typing (values:HeuristicDerived).
This RE-SOURCES from the authoritative text (fixes CSV omissions/structure loss).
Run:  python3 core-ontologies/tools/generate_n3_from_ohchr.py
"""
import re, os, glob, sys

AUTH = "core-ontologies/_authoritative"
OUT  = "core-ontologies/un-instruments"
BASE_URL = "https://www.ohchr.org/en/instruments-mechanisms/instruments/"
UDHR_URL = "https://www.ohchr.org/en/human-rights/universal-declaration/translations/english"

# OHCHR slugs SUPERSEDED by the cleaner, fully-segmented ICRC versions
# (gci/gcii/gciii/gciv-1949, api/apii-1977). Skipped so regen never re-creates the
# duplicates — the ICRC files are the canonical IHL copies.
SKIP_SLUGS = {
    "geneva-convention-relative-protection-civilian-persons-time-war",   # → gciv-1949
    "geneva-convention-relative-treatment-prisoners-war",                # → gciii-1949
    "protocol-additional-geneva-conventions-12-august-1949-and",         # → api-1977
    "protocol-additional-geneva-conventions-12-august-1949-and-0",       # → apii-1977
}

def slug_to_url(slug):
    return UDHR_URL if slug in ("english","udhr") else BASE_URL + slug
def norm_slug(slug):
    return "udhr" if slug == "english" else slug

def esc(s): return re.sub(r"\s+"," ", s.replace("\\","\\\\").replace('"','\\"')).strip()

# Scraper/nav boilerplate that must never survive into values:originalText.
_BOILERPLATE = ("Download: PDF", "Download:", "Table of Contents",
                "Print this page", "Share via email", "Share via Twitter",
                "Share via Facebook", "VIEW THIS PAGE IN")
def strip_boilerplate(t):
    for b in _BOILERPLATE:
        t = t.replace(b, " ")
    return re.sub(r"\s+", " ", t).strip()

def num(tok):
    if tok.isdigit(): return int(tok)
    rom = {'I':1,'V':5,'X':10,'L':50,'C':100,'D':500,'M':1000}
    t = tok.upper(); tot = 0
    for i,ch in enumerate(t):
        if ch not in rom: return 0
        v = rom[ch]
        tot += -v if (i+1 < len(t) and rom.get(t[i+1],0) > v) else v
    return tot

# Ordered keyword -> category (first match wins). Auto-assigned; flagged values:categoryStatus auto.
CATS = [
 ("International Humanitarian Law (Law of War)", ["geneva-convention","protocol-additional-geneva"]),
 ("International criminal justice", ["rome-statute","statute-international-criminal-tribunal","statute-international-tribunal","non-applicability-statutory-limitations","recruitment-use-financing"]),
 ("Genocide & atrocity prevention", ["crime-genocide"]),
 ("Enforced disappearance", ["enforced"]),
 ("Refugees & statelessness", ["status-refugees","relating-status-refugees","statelessness","stateless-persons"]),
 ("Slavery, forced labour & trafficking", ["slavery","forced-labour","trafficking","traffic-persons","smuggling-migrants","worst-forms-child-labour"]),
 ("Labour rights (ILO)", ["remuneration","employment-policy","discrimination-employment","freedom-association","right-organise","minimum-age-convention","collective-bargaining"]),
 ("Torture & ill-treatment", ["against-torture","subjected-torture","-torture"]),
 ("Administration of justice, detention & law enforcement", ["basic-principles","body-principles-protection-all-persons-under","code-conduct-law-enforcement","guidelines-role-prosecutors","guidelines-action-children-criminal","standard-minimum-rules","rules-protection-juveniles","rules-treatment-women-prisoners","non-custodial","prevention-juvenile-delinquency","safeguards-guaranteeing","principles-effective","principles-medical-ethics","principles-international-co-operation","justice-victims-crime"]),
 ("Persons with disabilities", ["disabilit","disabled-persons","mentally-retarded","equalization-opportunities","mental-illness"]),
 ("Migrants & non-nationals", ["migrant-workers","not-nationals"]),
 ("Women", ["against-women","discrimination-against-women","violence-against-women"]),
 ("Children", ["rights-child","sale-children","child"]),
 ("Equality & non-discrimination", ["racial","race-and-racial","discrimination-education","intolerance"]),
 ("Indigenous, minorities & self-determination", ["indigenous","national-or-ethnic","granting-independence-colonial","permanent","resolution-1803"]),
 ("Older persons", ["older-persons"]),
 ("Bioethics, science & culture", ["human-genome","cultural-diversity","scientific-and-technological"]),
 ("Development, peace, food & solidarity rights", ["right-development","peoples-peace","social-progress","eradication-hunger","millennium","hivaids"]),
 ("Human rights defenders & national institutions", ["right-and-responsibility-individuals","national-institutions"]),
 ("Marriage & family", ["consent-marriage","marriage"]),
 ("International Bill of Human Rights", ["udhr","human-rights-individuals","covenant-civil-and-political","covenant-economic-social","universal-declaration-human-rights"]),
 ("Foundational declarations", ["vienna-declaration"]),
]
def categorize(slug):
    for cat, kws in CATS:
        if any(k in slug for k in kws): return cat
    return "Other / thematic"

# title/date overrides for pages whose header format differs (e.g. UDHR translations page)
OVERRIDES = {
    "english": ("Universal Declaration of Human Rights", "1948-12-10"),
    "udhr":    ("Universal Declaration of Human Rights", "1948-12-10"),
}

def trim_content(text):
    """Drop site nav header (before the 'Human Rights Instruments' breadcrumb) and the
    footer ('Tags'/'VIEW THIS PAGE IN' onward) so only the instrument content remains."""
    t = text
    m = re.search(r"^Human Rights Instruments\s*$", t, re.M)
    if m: t = t[m.start():]
    fm = re.search(r"\n(VIEW THIS PAGE IN|Tags)\b", t)
    if fm: t = t[:fm.start()]
    return t

# lines that are page metadata, not provision content (for the paragraph fallback)
META = re.compile(r"^(Human Rights Instruments|UNIVERSAL INSTRUMENT|INTERNATIONAL INSTRUMENT|"
                  r"REGIONAL INSTRUMENT|ADOPTED|BY|Share|Download:|PDF|Table of Contents|"
                  r"Entry into force.*|General Assembly resolution.*|.*resolution \d+.*)\s*$", re.I)

def deontic(t):
    t = t.lower()
    if re.match(r"^\s*(no one|no party|no person|nothing)\b", t) or "shall be prohibited" in t \
       or "shall not" in t or re.search(r"\bmay not be\b", t):
        return ("values:Prohibition", "values:borneBy values:Agent")
    if "has the right" in t or "have the right" in t or "entitled to" in t or "right to" in t:
        return ("values:Right", "values:heldBy values:NaturalPerson")
    if re.search(r"\bshall (ensure|take|undertake|guarantee|recognize|recognise|respect|protect|promote|adopt|prohibit|provide)\b", t) \
       or "undertake to" in t or "shall be" in t:
        return ("values:Obligation", "values:borneBy values:Agent")
    return ("values:Undertaking", None)

def parse_instrument(text, slug):
    text = trim_content(text.replace("\r", ""))
    # title / date
    if slug in OVERRIDES:
        title, date = OVERRIDES[slug]
    else:
        title = norm_slug(slug).replace("-", " ").title(); date = ""
        lines = text.split("\n")
        for i, ln in enumerate(lines):
            if ln.strip().upper() == "ADOPTED":
                for j in range(i-1, -1, -1):
                    if lines[j].strip() and not META.match(lines[j].strip()):
                        title = lines[j].strip(); break
                for j in range(i+1, len(lines)):
                    if lines[j].strip():
                        date = lines[j].strip(); break
                break
    # Article N / Principle N headers
    hdr = re.compile(r"^(Article|Principle|Rule)\s+([IVXLCDM]+|\d+)\b[^\n]*$", re.M)
    marks = [(m.group(1), num(m.group(2)), m.start(), m.end()) for m in hdr.finditer(text) if num(m.group(2)) > 0]
    if marks:
        pre = ""
        pm = re.search(r"^Preamble\s*$", text, re.M)
        if pm:
            cand = text[pm.end():marks[0][2]].strip()
            if len(cand) > 40: pre = cand
        best = {}
        for i,(kind,n,s,e) in enumerate(marks):
            body = text[e: marks[i+1][2] if i+1 < len(marks) else len(text)].strip()
            k = (kind,n)
            if k not in best or len(body) > len(best[k][1]):
                best[k] = (kind, body)
        segs = sorted([(k[1], v[0], v[1]) for k,v in best.items()])
        return title, date, pre, segs
    # FALLBACK: no Article/Principle headers (operative declarations) -> paragraph segmentation
    body = text
    sm = re.search(r"^Share\s*$", body, re.M)
    if sm: body = body[sm.end():]
    paras = [re.sub(r"\s+", " ", p.strip()) for p in re.split(r"\n\s*\n", body) if p.strip()]
    paras = [p for p in paras if len(p) >= 30 and not META.match(p)]
    # if blank-line splitting barely worked, try splitting on numbered items (inline "1. ... 2. ...")
    if len(paras) <= 2:
        joined = re.sub(r"\s+", " ", body).strip()
        items = re.split(r"(?<![\d.])(?=\b\d{1,3}\.\s+[A-Z(])", joined)
        items = [x.strip() for x in items if len(x.strip()) >= 30 and not META.match(x.strip())]
        if len(items) > len(paras):
            paras = items
    segs = [(i+1, "Paragraph", p) for i, p in enumerate(paras)]
    return title, date, "", segs

def emit(slug, title, date, pre, segs):
    ns = "https://ns.webcivics.org/values/" + norm_slug(slug) + "#"
    L = ['@prefix rdf:    <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .',
         '@prefix rdfs:   <http://www.w3.org/2000/01/rdf-schema#> .',
         '@prefix dc:     <http://purl.org/dc/terms/> .',
         '@prefix values: <https://ns.webcivics.org/values/> .',
         f'@prefix doc:    <{ns}> .', '',
         '# ============================================================',
         f'# {title}',
         '# RE-SOURCED from authoritative OHCHR text (verbatim originalText).',
         '# Deontic typing = lexical heuristic (values:HeuristicDerived) — pending review.',
         '# states->parties + universalisation = curation overlay (amendedText), pending.',
         '# ============================================================', '',
         'doc:Instrument a values:ValuesCredential ;',
         f'    dc:title "{esc(title)}"@en ;']
    if re.search(r"\d{4}", date): L.append(f'    dc:date "{esc(date)}" ;')
    L.append(f'    values:category "{esc(categorize(norm_slug(slug)))}" ;')
    L.append('    values:categoryStatus values:AutoAssigned ;')
    L.append(f'    values:source <{slug_to_url(slug)}> ;')
    L.append('    rdfs:comment "Re-sourced from authoritative OHCHR text; deontic + amendment layers pending." .')
    L.append('')
    pre = strip_boilerplate(pre)
    if len(pre) > 40:
        L += ['doc:preamble a values:Undertaking ; values:partOf doc:Instrument ;',
              '    values:deonticStatus values:HeuristicDerived ;',
              f'    values:originalText "{esc(pre)}"@en .', '']
    for n, kind, body in segs:
        body = strip_boilerplate(body)
        if len(body) < 8: continue
        ty, bearer = deontic(body)
        L.append(f'doc:{kind.lower()}-{n} a {ty} ;')
        L.append('    values:partOf doc:Instrument ;')
        if bearer: L.append(f'    {bearer} ;')
        L.append(f'    dc:title "{kind} {n}"@en ;')
        L.append('    values:deonticStatus values:HeuristicDerived ;')
        L.append(f'    values:originalText "{esc(body)}"@en .')
        L.append('')
    return "\n".join(L)

def main():
    os.makedirs(OUT, exist_ok=True)
    items = []  # (slug, text)
    allf = os.path.join(AUTH, "ohchr_corpus.txt")
    if os.path.exists(allf):
        blob = open(allf, encoding="utf-8", errors="replace").read()
        parts = re.split(r"@@@SLUG:(.*?)@@@\n", blob)
        for i in range(1, len(parts), 2):
            items.append((parts[i].strip(), parts[i+1]))
    for f in glob.glob(os.path.join(AUTH, "ohchr__*.txt")):
        slug = os.path.basename(f)[len("ohchr__"):-4]
        if slug not in [s for s,_ in items]:
            items.append((slug, open(f, encoding="utf-8", errors="replace").read()))
    if not items:
        sys.exit("No authoritative text found in " + AUTH)
    idx = {}  # category -> [(title, slug, nprov, date)]
    for slug, text in items:
        if norm_slug(slug) in SKIP_SLUGS:
            continue  # superseded by the canonical ICRC version
        title, date, pre, segs = parse_instrument(text, slug)
        ns = norm_slug(slug)
        out = os.path.join(OUT, ns + ".n3")
        open(out, "w", encoding="utf-8").write(emit(slug, title, date, pre, segs))
        cat = categorize(ns)
        idx.setdefault(cat, []).append((title, ns, len(segs), date))
    # write categorised index
    lines = ["# Values Credentials — Categorised Index",
             f"\n**{len(items)} instruments** re-sourced from authoritative OHCHR text "
             "(`core-ontologies/un-instruments/<slug>.n3`). Categories auto-assigned (review-pending).\n"]
    for cat in sorted(idx):
        lines.append(f"\n## {cat}  ({len(idx[cat])})\n")
        for title, ns, n, date in sorted(idx[cat]):
            d = f" — {date}" if re.search(r"\d{4}", date or "") else ""
            lines.append(f"- **{title}**{d} · {n} provisions · `{ns}.n3`")
    open(os.path.join("core-ontologies", "INDEX.md"), "w", encoding="utf-8").write("\n".join(lines))
    print(f"\nGenerated {len(items)} instruments + core-ontologies/INDEX.md "
          f"({len(idx)} categories).")

if __name__ == "__main__":
    main()
