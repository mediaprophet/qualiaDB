#!/usr/bin/env python3
"""
legis2cml.py — Legislation PDF -> CML concept layer + *.cml.html, via a local Ollama model.

Emits the WebCivics / QualiaDB pattern (faithful to core-ontologies/cml.n3 and the
concepts/*.n3 corpus):

  TEXT (verbatim, immutable)  ->  cml:Concept (cml:realizedBy + cml:integrityHash)
                              ->  LOGIC: cml:asserts a values: norm (Obligation / Prohibition /
                                         Permission / Right / Undertaking), cml:modality cml:Deontic

THE CURATION PRIME DIRECTIVE (non-negotiable): a machine may only PROPOSE. Everything this tool
writes is cml:curationStatus cml:Proposed / values:deonticStatus values:HeuristicDerived. It never
asserts cml:Attested or skos:exactMatch — those are a signed, authoritative *human* action. Regenerating
is therefore non-destructive: it rewrites only the machine-proposed layer.

Small context windows: the document is split into PAGES and enriched ONE PAGE AT A TIME, appending
results to a <slug>.progress.json (resumable with --resume). Final artefacts are rendered from the
accumulated progress. Deterministic structure (provision boundaries + fragment ids) is parsed without
the LLM; the LLM only classifies the deontic character of provisions on each page.

Package (self-contained, references the original PDF):
  <out-dir>/
    <slug>.pdf            copy of the source (or a reference if --link) + sha256 in the manifest
    <slug>.cml.n3         concept layer (SOURCE-style, all cml:Proposed)
    <slug>.cml.html       CML-annotated human-readable rendering (RDFa; styled; fragment ids)
    <slug>.jsonld         the same graph as JSON-LD
    manifest.json         source ref + hash, schema version, model, counts, provenance
    <slug>.progress.json  per-page append log (resume state)

Install:  pip install pymupdf requests        (rdflib optional, for --emit-ttl)
          ollama pull llama3.1:8b
Usage:    python legis2cml.py --input privacy-act-1988.pdf \
              --title "Privacy Act 1988 (Cth)" --jurisdiction AU \
              --base-iri https://ns.webcivics.net/values/privacy-act-1988 --model llama3.1:8b
          python legis2cml.py --input act.pdf --no-llm     # structure only, fast

Authorship note: this tool is an instrument. It is not an author, and it cannot attest.
"""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import re
import shutil
import sys
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path

# --------------------------------------------------------------------------- namespaces
NS = {
    "rdf": "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
    "rdfs": "http://www.w3.org/2000/01/rdf-schema#",
    "skos": "http://www.w3.org/2004/02/skos/core#",
    "dc": "http://purl.org/dc/terms/",
    "prov": "http://www.w3.org/ns/prov#",
    "xsd": "http://www.w3.org/2001/XMLSchema#",
    "oa": "http://www.w3.org/ns/oa#",
    "cml": "https://ns.webcivics.net/cml/",
    "values": "https://ns.webcivics.net/values/",
    "concept": "https://ns.webcivics.net/concept/",
    # COF = Context Optimisation Format — agent-facing HTML+RDFa profile (not a second graph world)
    "cof": "https://ns.webcivics.net/cof/",
}
TOOL_IRI = "https://ns.webcivics.net/tools/legis2cml"
SCHEMA_DEFAULT = "2"
COF_PROFILE = "https://ns.webcivics.net/cof/profile/html-rdfa-1"

# values: deontic operators (the target vocabulary). Undertaking is the safe default.
DEONTIC = {
    "obligation": "values:Obligation",
    "prohibition": "values:Prohibition",
    "permission": "values:Permission",
    "right": "values:Right",
    "undertaking": "values:Undertaking",
}
BEARER = {  # values:Agent lattice
    "agent": "values:Agent", "state": "values:State", "legalperson": "values:LegalPerson",
    "naturalperson": "values:NaturalPerson", "artificialagent": "values:ArtificialAgent",
}


# --------------------------------------------------------------------------- data model
@dataclass
class Provision:
    frag: str
    kind: str            # "part" | "division" | "section"
    number: str
    heading: str
    text: str = ""
    start_page: int = 0
    integrity: str = ""  # sha256 of verbatim text


@dataclass
class Instrument:
    title: str
    slug: str
    jurisdiction: str
    base_iri: str
    eli: str | None
    provisions: list = field(default_factory=list)


# --------------------------------------------------------------------------- pdf + text
def extract_pages(pdf_path: Path) -> list[tuple[int, str]]:
    try:
        import fitz  # PyMuPDF
    except ImportError:
        sys.exit("PyMuPDF required: pip install pymupdf")
    doc = fitz.open(pdf_path)
    pages = [(i + 1, page.get_text("text")) for i, page in enumerate(doc)]
    doc.close()
    return pages


def clean(raw: str) -> str:
    raw = raw.replace("\r", "\n")
    raw = re.sub(r"(\w)-\n(\w)", r"\1\2", raw)
    out = []
    for ln in raw.split("\n"):
        s = ln.strip()
        if re.fullmatch(r"\d{1,4}", s):
            continue
        if re.search(r"(Authorised Version|Compilation No\.|Federal Register of Legislation|^Page\s+\d+)", s, re.I):
            continue
        out.append(s)
    return re.sub(r"\n{3,}", "\n\n", "\n".join(out))


def sha256(s: str) -> str:
    return hashlib.sha256(s.encode("utf-8")).hexdigest()


def slugify(s: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", s.lower()).strip("-")


# --------------------------------------------------------------------------- structure parse
RE_PART = re.compile(r"^(Part|PART)\s+([0-9IVXLC]+[A-Za-z]?)\b[\s—\-:.]*(.*)$")
RE_DIV = re.compile(r"^(Division|DIVISION)\s+([0-9]+[A-Za-z]?)\b[\s—\-:.]*(.*)$")
RE_SECTION = re.compile(r"^(\d+[A-Z]{0,2})\s+([A-Z][^\n]{2,140})$")


def parse_pages(pages: list[tuple[int, str]], title_hint: str | None) -> tuple[str, list[Provision]]:
    """One pass over the pages; sections accumulate across page breaks; record start_page."""
    provisions: list[Provision] = []
    title = title_hint
    cur: Provision | None = None
    buf: list[str] = []

    def flush():
        nonlocal cur, buf
        if cur is not None:
            cur.text = "\n".join(buf).strip()
            cur.integrity = sha256(cur.text) if cur.text else ""
            provisions.append(cur)
        cur, buf = None, []

    for page_no, raw in pages:
        for ln in clean(raw).split("\n"):
            s = ln.strip()
            if not title and re.search(r"\bAct\s+(No\.\s*\d+\s+of\s+)?\d{4}\b", s):
                title = s
            if not s:
                if cur is not None:
                    buf.append("")
                continue
            m = RE_PART.match(s)
            if m:
                flush()
                provisions.append(Provision(f"part-{slugify(m.group(2))}", "part",
                                            m.group(2), m.group(3).strip() or f"Part {m.group(2)}",
                                            start_page=page_no))
                continue
            m = RE_DIV.match(s)
            if m:
                flush()
                provisions.append(Provision(f"div-{slugify(m.group(2))}", "division",
                                            m.group(2), m.group(3).strip() or f"Division {m.group(2)}",
                                            start_page=page_no))
                continue
            m = RE_SECTION.match(s)
            if m and not s.startswith("("):
                flush()
                cur = Provision(f"sec-{slugify(m.group(1))}", "section",
                                m.group(1), m.group(2).strip(), start_page=page_no)
                continue
            if cur is not None:
                buf.append(s)
    flush()
    return title or "Untitled Legislative Instrument", provisions


# --------------------------------------------------------------------------- ollama (per page)
SYSTEM = (
    "You classify legislative provisions into a deontic model, output STRICT JSON only, no prose. "
    "deonticType MUST be one of: Obligation, Prohibition, Permission, Right, Undertaking. "
    "borneBy (for duties) and heldBy (for rights) SHOULD be one of: Agent, State, LegalPerson, "
    "NaturalPerson, ArtificialAgent, or null."
)


def classify_page(page_text: str, here: list[Provision], model: str, url: str, timeout: int = 120) -> dict:
    try:
        import requests
    except ImportError:
        sys.exit("requests required: pip install requests")
    if not here:
        return {}
    if len(page_text) > 6000:
        page_text = page_text[:6000] + " […truncated]"
    want = [{"number": p.number, "heading": p.heading} for p in here]
    user = (
        f"Page text:\n\"\"\"\n{page_text}\n\"\"\"\n\n"
        f"Classify each of these provisions appearing on this page:\n{json.dumps(want)}\n\n"
        "Return a JSON object mapping each provision number (string) to "
        '{"deonticType":..., "borneBy":..., "heldBy":..., "summary":..., '
        '"mustProvide":..., "conditions":[...], "crossReferences":[...]}.'
    )
    payload = {"model": model, "stream": False, "format": "json",
               "options": {"temperature": 0.1},
               "messages": [{"role": "system", "content": SYSTEM},
                            {"role": "user", "content": user}]}
    try:
        r = requests.post(f"{url.rstrip('/')}/api/chat", json=payload, timeout=timeout)
        r.raise_for_status()
        data = _loads(r.json().get("message", {}).get("content", ""))
        return data if isinstance(data, dict) else {}
    except Exception as e:  # noqa: BLE001
        print(f"  ! page classify failed: {e}", file=sys.stderr)
        return {}


def _loads(s: str):
    s = (s or "").strip()
    try:
        return json.loads(s)
    except json.JSONDecodeError:
        m = re.search(r"\{.*\}", s, re.S)
        if m:
            try:
                return json.loads(m.group(0))
            except json.JSONDecodeError:
                return None
    return None


# --------------------------------------------------------------------------- serialisers
def _lit(s: str) -> str:
    return (s or "").replace("\\", "\\\\").replace('"', '\\"').replace("\n", " ").strip()


def norm_type(cls: dict | None) -> str:
    if not cls:
        return "values:Undertaking"
    return DEONTIC.get(str(cls.get("deonticType", "")).lower(), "values:Undertaking")


def provision_n3(inst: Instrument, p: Provision, cls: dict | None, source_ref: str,
                 source_name: str, schema: str) -> str:
    cid = f"concept:{inst.slug}-{p.frag}"
    nid = f"{cid}-norm"
    L = [f"{cid} a cml:Concept ;",
         f'    skos:prefLabel "{_lit(p.number + " " + p.heading)}"@en ;',
         f"    cml:realizedBy doc:{p.frag} ;"]
    if p.integrity:
        L.append(f'    cml:integrityHash "{p.integrity}" ;')
    if cls and cls.get("summary"):
        L.append(f'    skos:note "{_lit(cls["summary"])}" ;')
    if cls and cls.get("mustProvide"):
        L.append(f'    dc:description "{_lit(cls["mustProvide"])}" ;')
    for ref in (cls.get("crossReferences") if cls else []) or []:
        L.append(f'    dc:references "{_lit(str(ref))}" ;')
    L += [f"    cml:curationStatus cml:Proposed ;",
          f'    cml:schemaVersion "{schema}" ;',
          f"    cml:proposedBy <{TOOL_IRI}> ;",
          f"    prov:wasDerivedFrom <{source_ref}> ;",
          f'    dc:source "{_lit(source_name)}, p.{p.start_page}" ;',
          f"    cml:asserts {nid} ."]
    dtype = norm_type(cls)
    N = [f"{nid} a {dtype} ;",
         f"    cml:modality cml:Deontic ;",
         f"    values:partOf {cid} ;"]
    if dtype in ("values:Obligation", "values:Prohibition", "values:Permission"):
        N.append("    values:borneBy values:Agent ;")
        bearer = BEARER.get(str((cls or {}).get("borneBy", "")).lower())
        if bearer == "values:State":
            N.append("    values:borneBy values:State ;")
        if cls and cls.get("borneBy"):
            N.append(f'    skos:scopeNote "borne by: {_lit(str(cls["borneBy"]))}" ;')
    elif dtype == "values:Right":
        held = BEARER.get(str((cls or {}).get("heldBy", "")).lower(), "values:NaturalPerson")
        N.append(f"    values:heldBy {held} ;")
        if cls and cls.get("heldBy"):
            N.append(f'    skos:scopeNote "held by: {_lit(str(cls["heldBy"]))}" ;')
    N += ["    values:deonticStatus values:HeuristicDerived ;",
          "    cml:curationStatus cml:Proposed ."]
    return "\n".join(L) + "\n" + "\n".join(N) + "\n"


def build_n3(inst: Instrument, cls_by_num: dict, source_ref: str, source_name: str, schema: str) -> str:
    doc_ns = f"{NS['values']}{inst.slug}#"
    head = [f"@prefix {k}: <{v}> ." for k, v in NS.items()]
    head.append(f"@prefix doc: <{doc_ns}> .")
    head += ["",
             f"# CML concept layer — {inst.title}",
             f"# MACHINE-DERIVED from the TEXT layer — cml:Proposed, values:HeuristicDerived.",
             f"# Pending human attestation (cml:Attested / skos:exactMatch). Generated by legis2cml.",
             ""]
    blocks = [provision_n3(inst, p, cls_by_num.get(p.number), source_ref, source_name, schema)
              for p in inst.provisions if p.kind == "section"]
    return "\n".join(head) + "\n" + "\n".join(blocks)


def build_jsonld(inst: Instrument, cls_by_num: dict, source_ref: str) -> dict:
    ctx = {**NS, "@base": NS["concept"]}
    nodes = []
    for p in inst.provisions:
        if p.kind != "section":
            continue
        cls = cls_by_num.get(p.number) or {}
        cid = f"concept:{inst.slug}-{p.frag}"
        concept = {"@id": cid, "@type": "cml:Concept",
                   "skos:prefLabel": f"{p.number} {p.heading}",
                   "cml:realizedBy": {"@id": f"{NS['values']}{inst.slug}#{p.frag}"},
                   "cml:curationStatus": {"@id": "cml:Proposed"},
                   "cml:proposedBy": {"@id": TOOL_IRI},
                   "prov:wasDerivedFrom": {"@id": source_ref},
                   "cml:asserts": {"@id": f"{cid}-norm"}}
        if p.integrity:
            concept["cml:integrityHash"] = p.integrity
        if cls.get("summary"):
            concept["skos:note"] = cls["summary"]
        norm = {"@id": f"{cid}-norm", "@type": norm_type(cls),
                "cml:modality": {"@id": "cml:Deontic"},
                "values:partOf": {"@id": cid},
                "values:deonticStatus": {"@id": "values:HeuristicDerived"},
                "cml:curationStatus": {"@id": "cml:Proposed"}}
        nodes.append(concept)
        nodes.append(norm)
    return {"@context": ctx, "@graph": nodes}


# --------------------------------------------------------------------------- html (*.cml.html)
CSS = """
:root{color-scheme:light dark;--fg:#1a1a1a;--bg:#fff;--muted:#666;--rule:#e4e4e4;--accent:#6a4c93;
--logic:#f4f1f7;--target:#fff6d6;--warn:#8a5a00}
@media(prefers-color-scheme:dark){:root{--fg:#e8e6e3;--bg:#161514;--muted:#a5a19b;--rule:#333;
--accent:#c3a6e6;--logic:#201d25;--target:#3a3320;--warn:#e0b050}}
*{box-sizing:border-box}body{max-width:52rem;margin:0 auto;padding:2rem 1.25rem 6rem;
font:16px/1.6 Georgia,serif;color:var(--fg);background:var(--bg)}
header.act{border-bottom:2px solid var(--accent);margin-bottom:1.5rem}
header.act h1{font-size:1.7rem;margin:0 0 .25rem}
.meta{color:var(--muted);font:.8rem system-ui,sans-serif}
.banner{background:var(--logic);border:1px solid var(--rule);border-left:3px solid var(--warn);
border-radius:0 6px 6px 0;padding:.6rem .9rem;margin:1rem 0;font:.82rem system-ui,sans-serif;color:var(--fg)}
h2.part,h2.division{font:600 .95rem system-ui,sans-serif;text-transform:uppercase;letter-spacing:.04em;
color:var(--accent);border-top:1px solid var(--rule);padding-top:1.3rem;margin-top:2rem}
section.concept{margin:1.7rem 0;scroll-margin-top:1rem}
section.concept>h3{font-size:1.12rem;margin:0 0 .4rem}
section.concept>h3 .num{color:var(--accent);margin-right:.5rem}
a.frag{color:var(--muted);text-decoration:none;font-size:.8em;opacity:0;margin-left:.4rem}
section.concept:hover a.frag{opacity:1}:target{background:var(--target);border-radius:4px}
.text{white-space:pre-wrap}
aside.logic{font:.8rem ui-monospace,Consolas,monospace;background:var(--logic);
border-left:3px solid var(--accent);border-radius:0 6px 6px 0;padding:.6rem .9rem;margin:.6rem 0}
.op{display:inline-block;background:var(--accent);color:#fff;padding:.05rem .5rem;border-radius:999px;
font:.72rem system-ui,sans-serif;text-transform:uppercase;letter-spacing:.03em}
.prop{color:var(--muted)}.status{color:var(--warn)}
"""


def esc(s: str) -> str:
    return html.escape(s or "", quote=True)


def logic_html(inst: Instrument, p: Provision, cls: dict | None) -> str:
    nid = f"concept:{inst.slug}-{p.frag}-norm"
    dtype = norm_type(cls)
    op = dtype.split(":")[1]
    rows = []
    if dtype in ("values:Obligation", "values:Prohibition", "values:Permission"):
        rows.append('<span class="prop">borne by</span> '
                    '<span property="values:borneBy" resource="values:Agent">values:Agent</span>')
        if cls and cls.get("borneBy"):
            rows.append(f'<span class="prop">(</span><span property="skos:scopeNote">{esc(str(cls["borneBy"]))}</span><span class="prop">)</span>')
    elif dtype == "values:Right":
        held = BEARER.get(str((cls or {}).get("heldBy", "")).lower(), "values:NaturalPerson")
        rows.append(f'<span class="prop">held by</span> <span property="values:heldBy" resource="{held}">{esc(held)}</span>')
    extra = ""
    if cls and cls.get("mustProvide"):
        extra += f'<div><span class="prop">must provide:</span> <span property="dc:description">{esc(str(cls["mustProvide"]))}</span></div>'
    if cls and cls.get("crossReferences"):
        extra += f'<div><span class="prop">refs:</span> {esc("; ".join(map(str, cls["crossReferences"])))}</div>'
    return (
        f'<aside class="logic" rel="cml:asserts" resource="{nid}">'
        f'<span typeof="{dtype}" about="{nid}">'
        f'<span class="op" property="cml:modality" content="cml:Deontic">{esc(op)} · Deontic</span> '
        f'{" ".join(rows)}{extra}'
        f'<div class="status" property="cml:curationStatus" content="cml:Proposed">'
        f'⚑ machine-proposed (cml:Proposed) · values:HeuristicDerived — pending human attestation</div>'
        f'<span property="values:deonticStatus" resource="values:HeuristicDerived"></span>'
        f'</span></aside>'
    )


def render_html(inst: Instrument, cls_by_num: dict, source_pdf: str, source_hash: str) -> str:
    """HTML+RDFa dual surface: human-readable CML studio page + COF agent payload.

    COF (Context Optimisation Format) here is *not* a parallel document model —
    it is a constrained HTML+RDFa *profile* over the same CML graph: structural
    cof:Document / cof:Section / cof:Block / cof:Claim / cof:Entity bindings with
    attributes (ref, confidence, page) and no presentation bloat beyond a thin CSS.
    """
    prefix = " ".join(f"{k}: {v}" for k, v in NS.items())
    prefix += f" doc: {NS['values']}{inst.slug}#"
    doc_about = f"{NS['values']}{inst.slug}"
    body = []
    last_page = None
    for p in inst.provisions:
        if p.kind in ("part", "division"):
            body.append(
                f'<section class="{p.kind}" id="{p.frag}" typeof="cof:Section" '
                f'property="cof:hasSection" resource="doc:{p.frag}">'
                f'<h2 property="cof:title dc:title">{esc(p.kind.title())} {esc(p.number)}'
                f'{" — " + esc(p.heading) if p.heading else ""}</h2></section>'
            )
            continue
        # Page boundary marker for agent spatial attention (no PDF coords yet —
        # page index is the honest spatial unit from text extractors).
        if p.start_page and p.start_page != last_page:
            last_page = p.start_page
            body.append(
                f'<div class="meta" typeof="cof:Page" property="cof:hasPage" '
                f'resource="doc:page-{p.start_page}" content="{p.start_page}">'
                f'Page <span property="cof:pageNumber">{p.start_page}</span></div>'
            )
        cid = f"concept:{inst.slug}-{p.frag}"
        cls = cls_by_num.get(p.number) or {}
        conf_attr = ""
        raw_conf = cls.get("confidence")
        if raw_conf is not None:
            try:
                conf_attr = f' data-confidence="{float(raw_conf):.2f}"'
            except (TypeError, ValueError):
                conf_attr = ""
        claim_id = f"{cid}-claim"
        body.append(
            f'<section class="concept" id="{p.frag}" '
            f'typeof="cml:Concept cof:Section" about="{cid}" resource="{cid}" '
            f'property="cof:hasSection" data-page="{p.start_page}"{conf_attr}>'
            f'<h3><span class="num" property="skos:prefLabel cof:title">'
            f'{esc(p.number)} {esc(p.heading)}</span>'
            f'<a class="frag" href="#{p.frag}">#</a></h3>'
            f'<div class="text" typeof="cof:Block" property="cof:hasBlock values:originalText" '
            f'resource="doc:{p.frag}-text" data-page="{p.start_page}">'
            f'<span typeof="cof:Claim" property="cof:hasClaim" about="{claim_id}" '
            f'resource="{claim_id}"{conf_attr}>'
            f'{esc(p.text)}</span></div>'
            f'{logic_html(inst, p, cls)}'
            f'<link rel="cml:realizedBy" href="doc:{p.frag}" />'
            f'</section>'
        )
    jsonld = json.dumps(build_jsonld(inst, cls_by_num, source_pdf), indent=2, ensure_ascii=False)
    return f"""<!DOCTYPE html>
<html lang="en" prefix="{esc(prefix)}">
<head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="cof-profile" content="{COF_PROFILE}">
<meta name="cml-schema" content="{SCHEMA_DEFAULT}">
<title>{esc(inst.title)} — CML · COF</title>
<style>{CSS}</style>
</head>
<body typeof="values:ValuesCredential cof:Document" about="{esc(doc_about)}"
      resource="{esc(doc_about)}" vocab="{NS['cof']}"
      data-source="{esc(source_pdf)}" data-sha256="{esc(source_hash)}">
<header class="act">
  <h1 property="dc:title cof:title">{esc(inst.title)}</h1>
  <div class="meta">Jurisdiction <span property="values:category">{esc(inst.jurisdiction)}</span>
    · <span property="cof:profile" content="{COF_PROFILE}">HTML+RDFa COF</span>
    · CML (TEXT → CONCEPT → LOGIC) · legis2cml + Ollama (optional)</div>
  <link rel="prov:wasDerivedFrom" href="{esc(source_pdf)}" />
</header>
<div class="banner">⚑ <strong>Machine-proposed (<code>cml:Proposed</code>).</strong> Every concept and norm
below was derived automatically and is <em>provisional</em>; only a signed, authoritative human action may
attest it (<code>cml:Attested</code> / <code>skos:exactMatch</code>). Source:
<code>{esc(source_pdf)}</code> · sha256 <code>{esc(source_hash[:16])}…</code>
· Agent payload: this file is the COF surface (attributes carry graph edges; body is readable text).</div>
<main property="cof:body">
{chr(10).join(body)}
</main>
<script type="application/ld+json">
{jsonld}
</script>
</body>
</html>
"""


# --------------------------------------------------------------------------- main
def main() -> None:
    ap = argparse.ArgumentParser(description="Legislation PDF -> CML concept layer + *.cml.html via Ollama.")
    ap.add_argument("--input", required=True, type=Path)
    ap.add_argument("--out-dir", type=Path, default=None, help="package dir (default: ./<slug>-cml)")
    ap.add_argument("--title", default=None)
    ap.add_argument("--jurisdiction", default="AU")
    ap.add_argument("--base-iri", default=None, help="instrument IRI (default values: + slug)")
    ap.add_argument("--eli", default=None)
    ap.add_argument("--model", default="llama3.1:8b")
    ap.add_argument("--ollama-url", default="http://localhost:11434")
    ap.add_argument("--schema-version", default=SCHEMA_DEFAULT)
    ap.add_argument("--no-llm", action="store_true", help="structure only (all norms -> Undertaking)")
    ap.add_argument("--resume", action="store_true", help="reuse existing progress.json; enrich only new pages")
    ap.add_argument("--link", action="store_true", help="reference the PDF in place, do not copy it")
    ap.add_argument("--emit-ttl", action="store_true", help="also write .ttl (needs rdflib)")
    args = ap.parse_args()

    if not args.input.exists():
        sys.exit(f"input not found: {args.input}")

    print(f"· reading {args.input.name}")
    pdf_bytes = args.input.read_bytes()
    source_hash = hashlib.sha256(pdf_bytes).hexdigest()
    pages = extract_pages(args.input)
    print(f"  {len(pages)} pages")

    title, provisions = parse_pages(pages, args.title)
    slug = slugify(re.sub(r"\s*No\.\s*\d+.*$", "", title))
    base_iri = args.base_iri or f"{NS['values']}{slug}"
    inst = Instrument(title, slug, args.jurisdiction, base_iri, args.eli, provisions)
    sections = [p for p in provisions if p.kind == "section"]
    print(f"· parsed: {len(sections)} sections, {len(provisions) - len(sections)} headings")

    out_dir = args.out_dir or Path(f"./{slug}-cml")
    out_dir.mkdir(parents=True, exist_ok=True)
    progress_path = out_dir / f"{slug}.progress.json"

    progress = {"pages_done": [], "classifications": {}}
    if args.resume and progress_path.exists():
        progress = json.loads(progress_path.read_text(encoding="utf-8"))
        print(f"· resuming: {len(progress['pages_done'])} pages already done")

    if not args.no_llm:
        by_page: dict[int, list[Provision]] = {}
        for p in sections:
            by_page.setdefault(p.start_page, []).append(p)
        print(f"· enriching page-by-page via Ollama ({args.model})")
        for page_no, raw in pages:
            if page_no in progress["pages_done"]:
                continue
            here = by_page.get(page_no, [])
            if here:
                res = classify_page(clean(raw), here, args.model, args.ollama_url)
                for num, cls in (res or {}).items():
                    progress["classifications"][str(num)] = cls
                got = sum(1 for p in here if str(p.number) in progress["classifications"])
                print(f"  [p.{page_no}] {len(here)} provision(s) -> {got} classified")
            progress["pages_done"].append(page_no)
            progress_path.write_text(json.dumps(progress, indent=2), encoding="utf-8")  # APPEND checkpoint

    cls_by_num = progress["classifications"]

    # --- package ---
    if args.link:
        source_ref = args.input.resolve().as_uri()
        source_name = args.input.name
    else:
        dest_pdf = out_dir / f"{slug}.pdf"
        shutil.copyfile(args.input, dest_pdf)
        source_ref = f"{slug}.pdf"
        source_name = f"{slug}.pdf"

    (out_dir / f"{slug}.cml.n3").write_text(
        build_n3(inst, cls_by_num, source_ref, source_name, args.schema_version), encoding="utf-8")
    (out_dir / f"{slug}.cml.html").write_text(
        render_html(inst, cls_by_num, source_name, source_hash), encoding="utf-8")
    (out_dir / f"{slug}.jsonld").write_text(
        json.dumps(build_jsonld(inst, cls_by_num, source_ref), indent=2, ensure_ascii=False), encoding="utf-8")

    manifest = {
        "title": inst.title, "slug": slug, "jurisdiction": inst.jurisdiction,
        "baseIri": base_iri, "eli": inst.eli, "schemaVersion": args.schema_version,
        "curationStatus": "cml:Proposed",
        "generatedBy": {"tool": TOOL_IRI, "model": None if args.no_llm else args.model},
        "generatedAt": datetime.now(timezone.utc).isoformat(),
        "source": {"file": source_ref, "originalName": args.input.name,
                   "sha256": source_hash, "pages": len(pages), "linked": bool(args.link)},
        "counts": {"provisions": len(sections),
                   "classified": sum(1 for p in sections if str(p.number) in cls_by_num)},
        "files": [f"{slug}.cml.n3", f"{slug}.cml.html", f"{slug}.jsonld",
                  *([] if args.link else [f"{slug}.pdf"])],
        "note": "Machine-proposed layer. Regeneration is non-destructive; human cml:Attested overlays are never written by this tool.",
    }
    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")

    if args.emit_ttl:
        try:
            from rdflib import Graph
            g = Graph()
            g.parse(data=json.dumps(build_jsonld(inst, cls_by_num, source_ref)), format="json-ld")
            (out_dir / f"{slug}.ttl").write_text(g.serialize(format="turtle"), encoding="utf-8")
        except ImportError:
            print("  ! rdflib not installed; skipping .ttl")

    print(f"· package written to {out_dir}/  ({manifest['counts']['classified']}/{len(sections)} classified)")


if __name__ == "__main__":
    main()
