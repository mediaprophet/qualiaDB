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

Small context windows: the document is split at PROVISION/PARAGRAPH boundaries into bounded,
content-addressed segments. Long provisions use overlap so no clause disappears at a segment edge.
Successful segments are atomically checkpointed; failed segments remain pending for --resume.

Package (self-contained, references the original PDF):
  <out-dir>/
    <slug>.pdf            copy of the source (or a reference if --link) + sha256 in the manifest
    <slug>.cml.n3         concept layer (SOURCE-style, all cml:Proposed)
    <slug>.cml.html       CML-annotated human-readable rendering (RDFa; styled; fragment ids)
    <slug>.jsonld         the same graph as JSON-LD
    manifest.json         source ref + hash, schema version, model, counts, provenance
    <slug>.progress.json  atomic content-addressed segment checkpoints

Install:  pip install pymupdf requests        (rdflib optional, for --emit-ttl)
          ollama pull llama3.2:3b-instruct-q4_K_M
Usage:    python legis2cml.py --input privacy-act-1988.pdf \
              --title "Privacy Act 1988 (Cth)" --jurisdiction AU \
              --base-iri https://ns.webcivics.net/values/privacy-act-1988 \
              --model llama3.2:3b-instruct-q4_K_M
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
import subprocess
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
PROGRESS_VERSION = 6
DEFAULT_SEGMENT_CHARS = 8000
DEFAULT_SEGMENT_OVERLAP = 500
DEFAULT_SEGMENT_ITEMS = 1
MIN_LOGIC_CONFIDENCE = 0.5
NUM_PREDICT = 1800   # per-segment output-token cap
# Ollama ignores the model's 131072 architecture window and defaults num_ctx to 2048, then
# truncates any longer prompt FROM THE LEFT — silently dropping the system instructions/excerpt.
# The window is therefore sized explicitly from the segment budget (see choose_num_ctx). 8192 is
# the floor: it comfortably holds the default 8000-char excerpt (~2500 prompt tokens) plus output.
MIN_NUM_CTX = 8192

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

# CML routing term -> implemented QualiaDB execution surface. The model selects only applicable
# entries; this registry, not model free text, controls what reaches RDF/N3/CogAI output.
LOGIC_SUITE = {
    "Deontic": ("cml:Deontic", "modalities::logic::deontic"),
    "Epistemic": ("cml:Epistemic", "modalities::epistemic"),
    "LTL": ("cml:LTL", "modalities::temporal_ltl"),
    "Paraconsistent": ("cml:Paraconsistent", "modalities::paraconsistent"),
    "ASP": ("cml:AnswerSetProgramming", "modalities::asp"),
    "Dialectical": ("cml:Dialectical", "modalities::dialectical"),
    "LinearLogic": ("cml:LinearLogic", "modalities::linear"),
    "DescriptionLogic": ("cml:DescriptionLogic", "modalities::dl"),
    "Argumentation": ("cml:Argumentation", "modalities::argumentation"),
    "AllenInterval": ("cml:AllenInterval", "modalities::interval_reasoning"),
    "Diffusion": ("cml:Diffusion", "modalities::diffusion"),
    "CogAI": ("cml:CogAI", "sparql_library::parsers::chk_parser"),
    "SHACL": ("cml:SHACL", "modalities::logic::shacl"),
    "N3Logic": ("cml:N3Logic", "modalities::logic::n3_compiler"),
}


# --------------------------------------------------------------------------- data model
@dataclass
class Provision:
    frag: str
    kind: str            # "part" | "division" | "schedule" | "section" | "subsection"
    number: str
    heading: str
    text: str = ""
    start_page: int = 0
    integrity: str = ""  # sha256 of verbatim text
    parent: str | None = None  # parent section frag, for subsections
    # Full body before subsection split (containers keep this so N3/HTML never lose text).
    full_text: str = ""


@dataclass
class Instrument:
    title: str
    slug: str
    jurisdiction: str
    base_iri: str
    eli: str | None
    provisions: list = field(default_factory=list)
    act_no: str | None = None
    year: str | None = None
    long_title: str | None = None   # the "An Act to…" intro
    date: str | None = None         # compilation / in-force date, when stated


@dataclass
class Segment:
    segment_id: str
    items: list[dict]

    @property
    def char_count(self) -> int:
        return sum(len(item["text"]) for item in self.items)


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
        if re.search(r"\bAct\s+\d{4}\b.*\bNo\.\s*\d+", s, re.I):
            continue
        if re.search(r"\.{5,}\s*\d*\s*$", s):
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
# A real Schedule heading is "Schedule N—Title" (em/en dash). Requiring the dash avoids matching
# commencement-table rows and cross-references that merely start with "Schedule N".
RE_SCHEDULE = re.compile(r"^(Schedule|SCHEDULE)\s+([0-9]+[A-Za-z]?)\s*[—–]\s*(.+)$")
# Section heading: number + title. Title may be Title Case or ALL CAPS. Reject lowercase-start
# titles (those are almost always cross-references / prose, not arrangement headings).
RE_SECTION = re.compile(r"^(\d+[A-Z]{0,2})\s+([A-Z0-9][^\n]{1,160})$")
# Soft form: older Commonwealth numbering "1.—(1.) This Act…" / "2. Section eighty-six…".
# Requires a period after the number. Body may start with a subsection marker "(1.)".
RE_HISTORICAL_SECTION = re.compile(r"^(\d+[A-Z]{0,2})\.\s*[—–\-]?\s*(.+)$")
RE_EU_CHAPTER = re.compile(r"^CHAPTER\s+([IVXLC]+)$", re.I)
RE_EU_ARTICLE = re.compile(r"^Article\s+(\d+[A-Z]?)$", re.I)
# The Commonwealth enacting formula ends the front matter (cover + Contents/arrangement) and
# begins the operative text. When present, the table of contents before it is skipped: parsing it
# as body produced phantom empty Parts/Divisions and collided the arrangement's section numbers
# with the real ones (an amendment item "1" became sec-1-2 alongside the principal section 1).
RE_ENACTING = re.compile(r"Parliament of Australia enacts|BE IT ENACTED", re.I)


def _looks_like_section_heading(title: str) -> bool:
    """Reject prose/cross-refs that begin like a section number but are not headings."""
    t = title.strip()
    if not t or len(t) < 2:
        return False
    # "of Part III", "or 40 to give information" — not a section title.
    if t[:1].islower():
        return False
    if re.match(r"^(of|or|and|to|in|for|as|under|made by|has no effect)\b", t, re.I):
        return False
    # Years alone / table fragments.
    if re.fullmatch(r"\d{4}", t):
        return False
    words = t.split()
    # Real AU section titles are short noun phrases ("Short title", "Commencement").
    # A long multi-word sentence is body text that must stay inside the current provision.
    if len(words) > 12:
        return False
    if t.endswith(".") and len(words) > 4:
        return False
    # Body openers that appear after "N." in OCR'd paragraphs.
    if re.match(r"^(If|Subject|This|When|Where|Unless|Despite|For the purposes)\b", t):
        return False
    return True


def provision_source_text(p: Provision) -> str:
    """Text that must appear in exported graphs: prefer full pre-split body for containers."""
    if (p.full_text or "").strip():
        return p.full_text
    return p.text or ""


def coverage_report(inst: Instrument) -> dict:
    """Integrity metrics so empty/truncated packages are visible in manifest.json."""
    concepts = concept_units(inst.provisions)
    with_text = [p for p in concepts if provision_source_text(p).strip()]
    empty = [p for p in concepts if not provision_source_text(p).strip()]
    structural = [p for p in inst.provisions if p.kind in ("part", "division", "schedule")]
    return {
        "concepts": len(concepts),
        "conceptsWithText": len(with_text),
        "emptyConcepts": len(empty),
        "emptyFrags": [p.frag for p in empty[:50]],
        "structural": len(structural),
        "textCoverageRatio": (len(with_text) / len(concepts)) if concepts else 1.0,
        "ok": (len(empty) == 0) or (len(with_text) / max(len(concepts), 1) >= 0.85),
    }


def parse_pages(pages: list[tuple[int, str]], title_hint: str | None) -> tuple[str, list[Provision]]:
    """One pass over the pages; sections accumulate across page breaks; record start_page."""
    provisions: list[Provision] = []
    title = title_hint or infer_title(pages)
    cur: Provision | None = None
    buf: list[str] = []
    frag_counts: dict[str, int] = {}
    current_schedule: str | None = None
    pending_eu_chapter: tuple[str, int] | None = None
    pending_eu_article: tuple[str, int] | None = None

    def unique_frag(base: str) -> str:
        count = frag_counts.get(base, 0) + 1
        frag_counts[base] = count
        return base if count == 1 else f"{base}-{count}"

    def scoped(base: str) -> str:
        # Fragments inside a Schedule are namespaced to it so amendment-item numbers never
        # collide with the principal instrument's own Part/Division/section numbers.
        return unique_frag(f"sch-{current_schedule}-{base}" if current_schedule else base)

    def is_heading(match: "re.Match") -> bool:
        # A real Part/Division heading has an empty or capitalised title. A line like
        # "Division 2 of Part III covers…" or "Part XI of the Crimes Act 1914" is prose or a
        # cross-reference (its title starts lowercase) and must not become a structural node.
        head = match.group(3).strip()
        return not head[:1].islower()

    def flush():
        nonlocal cur, buf
        if cur is not None:
            cur.text = "\n".join(buf).strip()
            cur.integrity = sha256(cur.text) if cur.text else ""
            provisions.append(cur)
        cur, buf = None, []

    # Provisions begin only after the enacting formula (if the instrument has one); before it we
    # still scan for the title but emit nothing, so the arrangement/contents pages are skipped.
    cleaned_lines = [ln for _pg, raw in pages for ln in clean(raw).split("\n")]
    has_enacting_formula = any(RE_ENACTING.search(ln) for ln in cleaned_lines)
    has_eu_articles = any(RE_EU_ARTICLE.match(ln.strip()) for ln in cleaned_lines)
    # EU Regulations place extensive recitals before the operative Chapters/Articles. They remain
    # in the source PDF but are not emitted as operative norms by this legislation pass.
    in_body = not (has_enacting_formula or has_eu_articles)

    for page_no, raw in pages:
        for ln in clean(raw).split("\n"):
            s = ln.strip()
            if (not title and not re.match(r"^[\d(]", s)
                    and re.search(r"\bAct\s+(No\.\s*\d+\s+of\s+)?\d{4}\b", s)):
                title = s  # last-ditch: a heading-like line, never a numbered provision/body clause
            if not in_body:
                if RE_ENACTING.search(s):
                    in_body = True
                    continue
                if RE_EU_CHAPTER.match(s) or RE_EU_ARTICLE.match(s):
                    in_body = True
                else:
                    continue
            if not s:
                if cur is not None:
                    buf.append("")
                continue
            if pending_eu_chapter is not None:
                number, start_page = pending_eu_chapter
                provisions.append(Provision(unique_frag(f"chapter-{slugify(number)}"), "part",
                                            number, s, start_page=start_page))
                pending_eu_chapter = None
                continue
            if pending_eu_article is not None:
                number, start_page = pending_eu_article
                cur = Provision(unique_frag(f"article-{slugify(number)}"), "section",
                                number, s, start_page=start_page)
                pending_eu_article = None
                continue
            m = RE_EU_CHAPTER.match(s)
            if m:
                flush()
                current_schedule = None
                pending_eu_chapter = (m.group(1).upper(), page_no)
                continue
            m = RE_EU_ARTICLE.match(s)
            if m:
                flush()
                pending_eu_article = (m.group(1).upper(), page_no)
                continue
            m = RE_SCHEDULE.match(s)
            if m:
                sched = slugify(m.group(2))
                # A running "Schedule N—…" page header repeats on every page of the schedule;
                # only the first (a genuine new schedule number) starts a schedule and flushes.
                if sched != current_schedule:
                    flush()
                    current_schedule = sched
                    provisions.append(Provision(unique_frag(f"sch-{sched}"), "schedule",
                                                m.group(2), m.group(3).strip() or f"Schedule {m.group(2)}",
                                                start_page=page_no))
                continue
            m = RE_PART.match(s)
            if m and is_heading(m):
                flush()
                provisions.append(Provision(scoped(f"part-{slugify(m.group(2))}"), "part",
                                            m.group(2), m.group(3).strip() or f"Part {m.group(2)}",
                                            start_page=page_no))
                continue
            m = RE_DIV.match(s)
            if m and is_heading(m):
                flush()
                provisions.append(Provision(scoped(f"div-{slugify(m.group(2))}"), "division",
                                            m.group(2), m.group(3).strip() or f"Division {m.group(2)}",
                                            start_page=page_no))
                continue
            m = RE_SECTION.match(s)
            if m and not s.startswith("(") and _looks_like_section_heading(m.group(2)):
                flush()
                cur = Provision(scoped(f"sec-{slugify(m.group(1))}"), "section",
                                m.group(1), m.group(2).strip(), start_page=page_no)
                continue
            m = RE_HISTORICAL_SECTION.match(s)
            # Prefer RE_SECTION when both match; historical is dotted "N. …" forms.
            if m and not has_eu_articles and not s.startswith("(") and not RE_SECTION.match(s):
                rest = m.group(2).strip()
                # Reject obvious non-headings that happen to start a line with "N." inside body
                # only when we have no open section — otherwise still open a section (historical
                # Acts put the whole clause after the number).
                flush()
                num = m.group(1)
                # Inline subsection start "1.—(1.) This Act…" or long prose → body, generic title.
                if (rest.startswith("(")
                        or len(rest) > 90
                        or (rest[:1].islower() if rest else True)
                        or re.match(
                            r"^(If|Subject|This|When|Where|Unless|Despite|For the purposes|"
                            r"Section |The |A |An )\b",
                            rest,
                        )
                        or not _looks_like_section_heading(rest)):
                    cur = Provision(scoped(f"sec-{slugify(num)}"), "section",
                                    num, f"Section {num}", start_page=page_no)
                    buf.append(rest)
                else:
                    cur = Provision(scoped(f"sec-{slugify(num)}"), "section",
                                    num, rest, start_page=page_no)
                continue
            if cur is not None:
                buf.append(s)
    flush()
    if pending_eu_chapter is not None:
        number, start_page = pending_eu_chapter
        provisions.append(Provision(unique_frag(f"chapter-{slugify(number)}"), "part",
                                    number, f"Chapter {number}", start_page=start_page))
    if pending_eu_article is not None:
        number, start_page = pending_eu_article
        provisions.append(Provision(unique_frag(f"article-{slugify(number)}"), "section",
                                    number, f"Article {number}", start_page=start_page))
    return title or "Untitled Legislative Instrument", provisions


def _clean_title(s: str) -> str:
    s = re.sub(r"\s+", " ", s).strip().strip(".,;:").strip()
    if s and s == s.upper():   # ALL-CAPS cover heading -> Title Case
        s = s.title()
    return s


# A cover line is not part of the title: the Act number, the long title, an assent stamp, the
# enacting words, a contents/ToC marker, or a SCALEplus/register note or URL. A bare year is NOT
# excluded here — it may be the title's own year ("… Act (No. 2) 1989"); a leading page-date year
# is stripped from the assembled title instead.
_NOT_TITLE_LINE = re.compile(
    r"^(No\.\s*\d|An Act\b|\[|BE IT\b|E it\b|CONTENTS?$|TABLE OF\b|\(?https?:|Note:|An electronic"
    r"|Prepared by\b|Office of Parliamentary|Compilation\b|Includes amendment|No table of contents)",
    re.I)


def infer_title(pages: list[tuple[int, str]]) -> str | None:
    """Recover the Commonwealth citation title, robust to old scans and OCR noise."""
    joined = re.sub(r"\s+", " ", " ".join(raw for _page_no, raw in pages[:5]))
    # 1. Explicit citation: "… may be cited as the <Name> Act <year>".
    citation = re.search(r"may be cited as the\s+(.{2,120}?\bAct\b[^.]{0,40}?\d{4})\b", joined, re.I)
    if citation:
        return _clean_title(citation.group(1))
    # 2. Cover heading: the lines immediately above the first "No. X of YYYY" (or "No. X.").
    for _page_no, raw in pages[:3]:
        lines = [line.strip() for line in raw.replace("\r", "\n").split("\n") if line.strip()]
        for index, line in enumerate(lines):
            if not re.match(r"^No\.\s*\d+\b", line, re.I):
                continue
            head: list[str] = []
            for prev in reversed(lines[max(0, index - 5):index]):
                if _NOT_TITLE_LINE.match(prev):
                    break  # the title block is contiguous above "No. X"; stop at the first non-title line
                head.insert(0, prev)
                if len(" ".join(head)) > 90:
                    break
            candidate = _clean_title(" ".join(head))
            candidate = re.sub(r"^\d{4}[.,]?\s+", "", candidate).strip()  # drop a leading page-date year
            if len(candidate) >= 3 and re.search(r"[A-Za-z]", candidate):
                return candidate
    # 3. Long title as a last resort ("An Act to … purposes").
    long_title = re.search(r"\b(An Act\b.{5,120}?)(?:\bpurposes?\b|\.)", joined, re.I)
    if long_title:
        return _clean_title(long_title.group(1))
    return None


# --------------------------------------------------------------------------- subsections + metadata
RE_SUBSECTION = re.compile(r"^\((\d+[A-Za-z]?)\)\s+(.+)$")   # "(1)", "(2A)" subsection markers
RE_ACT_NO = re.compile(r"\bNo\.\s*(\d+)\s*(?:of\s+|,\s*)(\d{4})\b", re.I)
RE_INFORCE_DATE = re.compile(r"in force on\s+(\d{1,2}\s+[A-Za-z]+\s+\d{4})", re.I)
# Commonwealth long titles read "An Act to … , and for … purposes" — stop at "purposes" or the
# first full stop, so the citation/short-title text that follows is not swept in.
RE_LONG_TITLE = re.compile(r"\b(An Act\b.{5,240}?(?:\bpurposes?\b|\.))", re.I)


def split_subsections(section: Provision) -> list[Provision]:
    """Decompose a section's text into subsection provisions at whole '(N)' boundaries.

    Lead-in text before the first subsection stays on the section (its own text is trimmed to it).
    Returns [] when the section has fewer than two subsections — a single or no '(N)' is not worth
    splitting, and the section stays the classified unit. Each subsection carries its parent frag.
    """
    lines = section.text.split("\n")
    starts = [i for i, ln in enumerate(lines) if RE_SUBSECTION.match(ln)]
    if len(starts) < 2:
        return []
    subs: list[Provision] = []
    for k, start in enumerate(starts):
        end = starts[k + 1] if k + 1 < len(starts) else len(lines)
        block = "\n".join(lines[start:end]).strip()
        num = RE_SUBSECTION.match(lines[start]).group(1)
        frag = f"{section.frag}-ss-{slugify(num)}"
        subs.append(Provision(frag, "subsection", f"{section.number}({num})", section.heading,
                              block, section.start_page, sha256(block) if block else "",
                              parent=section.frag))
    lead = "\n".join(lines[:starts[0]]).strip()
    section.text = lead
    section.integrity = sha256(lead) if lead else ""
    return subs


def decompose_provisions(provisions: list[Provision]) -> list[Provision]:
    """Expand sections into section + subsection provisions in document order.

    Preserves each section's pre-split body on ``full_text`` so graph exports never drop
    subsection content when the parent becomes a container (lead-in only in ``text``).
    """
    out: list[Provision] = []
    for provision in provisions:
        if provision.kind == "section" and not provision.full_text:
            provision.full_text = provision.text or ""
            if provision.full_text and not provision.integrity:
                provision.integrity = sha256(provision.full_text)
        out.append(provision)
        if provision.kind == "section":
            out.extend(split_subsections(provision))
    return out


def children_of(provisions: list[Provision]) -> dict[str, list[str]]:
    """Map each container section frag to its ordered subsection frags."""
    kids: dict[str, list[str]] = {}
    for provision in provisions:
        if provision.kind == "subsection" and provision.parent:
            kids.setdefault(provision.parent, []).append(provision.frag)
    return kids


def classifiable_units(provisions: list[Provision]) -> list[Provision]:
    """Units sent to the model: subsections, plus leaf sections that have no subsections."""
    containers = {p.parent for p in provisions if p.kind == "subsection"}
    return [p for p in provisions
            if p.kind == "subsection" or (p.kind == "section" and p.frag not in containers)]


def concept_units(provisions: list[Provision]) -> list[Provision]:
    """Every provision that becomes a cml:Concept (sections — leaf or container — and subsections)."""
    return [p for p in provisions if p.kind in ("section", "subsection")]


def extract_metadata(pages: list[tuple[int, str]], title: str) -> dict:
    """Structured instrument metadata from the front matter (not sent to the model)."""
    front = " ".join(clean(raw) for _pg, raw in pages[:6])
    front = re.sub(r"\s+", " ", front)
    md: dict = {}
    m = RE_ACT_NO.search(front)
    if m:
        md["act_no"], md["year"] = m.group(1), m.group(2)
    else:
        ym = re.search(r"\bAct\s+(\d{4})\b", title or "")
        if ym:
            md["year"] = ym.group(1)
    m = RE_INFORCE_DATE.search(front)
    if m:
        md["date"] = m.group(1).strip()
    m = RE_LONG_TITLE.search(front)
    if m:
        md["long_title"] = re.sub(r"\s+", " ", m.group(1)).strip()
    return md


# --------------------------------------------------------------------------- bounded Ollama segments
SYSTEM = (
    "You classify legislative provisions into a deontic model, output STRICT JSON only, no prose. "
    "deonticType MUST be one of: Obligation, Prohibition, Permission, Right, Undertaking. "
    "borneBy (for duties) and heldBy (for rights) SHOULD be one of: Agent, State, LegalPerson, "
    "NaturalPerson, ArtificialAgent, or null. Classify the legal effect, not isolated modal words. "
    "Use Undertaking for titles, commencement, definitions, descriptive clauses, amendment machinery, "
    "and provisions that do not clearly impose a duty, ban conduct, grant permission, or create a right. "
    "A citation using 'may be cited' is Undertaking, not Permission. Conditional commencement is "
    "Undertaking, not Prohibition. DescriptionLogic applies only to explicit class/type/subclass "
    "hierarchies, not ordinary definitions or amendment wording. AllenInterval requires a relation "
    "between identifiable temporal intervals. LTL requires an ordered or recurring trace. "
    "Do not invent a bearer, holder, condition, cross-reference, premise, or conclusion."
)


def split_text(text: str, max_chars: int, overlap_chars: int) -> list[str]:
    """Split without dropping text, preferring paragraph and sentence boundaries."""
    text = text.strip()
    if not text:
        return [""]
    if max_chars < 1000:
        raise ValueError("max_chars must be at least 1000")
    if overlap_chars < 0 or overlap_chars >= max_chars:
        raise ValueError("overlap_chars must be >= 0 and smaller than max_chars")
    chunks: list[str] = []
    start = 0
    while start < len(text):
        limit = min(start + max_chars, len(text))
        end = limit
        if limit < len(text):
            floor = start + max_chars // 2
            candidates = [text.rfind("\n\n", floor, limit), text.rfind("\n", floor, limit),
                          text.rfind(". ", floor, limit)]
            boundary = max(candidates)
            if boundary >= floor:
                end = boundary + (2 if text[boundary:boundary + 2] == ". " else 0)
        chunk = text[start:end].strip()
        if chunk:
            chunks.append(chunk)
        if end >= len(text):
            break
        start = max(start + 1, end - overlap_chars)
    return chunks


def build_segments(provisions: list[Provision], max_chars: int, overlap_chars: int,
                   max_items: int = DEFAULT_SEGMENT_ITEMS) -> list[Segment]:
    """Pack complete short provisions together; split only provisions over budget."""
    groups: list[list[dict]] = []
    current: list[dict] = []
    current_chars = 0

    def flush() -> None:
        nonlocal current, current_chars
        if current:
            groups.append(current)
        current, current_chars = [], 0

    for provision in provisions:
        chunks = split_text(provision.text, max_chars, overlap_chars)
        for part, chunk in enumerate(chunks, 1):
            item = {"key": provision.frag, "number": provision.number,
                    "heading": provision.heading, "page": provision.start_page,
                    "part": part, "parts": len(chunks), "text": chunk}
            cost = len(chunk) + len(provision.heading) + 160
            if len(chunks) > 1:
                flush()
                groups.append([item])
            else:
                if current and (current_chars + cost > max_chars or len(current) >= max_items):
                    flush()
                current.append(item)
                current_chars += cost
    flush()

    segments = []
    for items in groups:
        canonical = json.dumps(items, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        segment_id = hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:24]
        segments.append(Segment(segment_id, items))
    return segments


def _normalise_classification(value: object) -> dict | None:
    if not isinstance(value, dict):
        return None
    dtype = str(value.get("deonticType", "")).strip().lower()
    if dtype not in DEONTIC:
        return None
    out: dict = {"deonticType": dtype.title()}
    for key in ("borneBy", "heldBy", "summary", "mustProvide"):
        raw = value.get(key)
        if raw is None or isinstance(raw, bool):
            continue
        cleaned = str(raw).strip()
        if cleaned.lower() in ("", "false", "null", "none", "n/a"):
            continue
        if key in ("borneBy", "heldBy") and cleaned.lower() not in BEARER:
            continue
        out[key] = cleaned[:2000]
    for key in ("conditions", "crossReferences"):
        raw = value.get(key, [])
        if raw is None:
            raw = []
        if not isinstance(raw, list):
            return None
        out[key] = [str(item).strip()[:500] for item in raw[:50] if str(item).strip()]
    try:
        out["confidence"] = max(0.0, min(1.0, float(value.get("confidence", 0.5))))
    except (TypeError, ValueError):
        out["confidence"] = 0.5
    applications = value.get("logicApplications", [])
    if applications is None:
        applications = []
    if not isinstance(applications, list):
        return None
    out["logicApplications"] = []
    for application in applications[:24]:
        if not isinstance(application, dict):
            return None
        logic = str(application.get("logic", "")).strip()
        summary = str(application.get("summary", "")).strip()
        if logic not in LOGIC_SUITE:
            return None
        if not summary:
            # A contentless application (empty summary, typically confidence 0.0 on amendment
            # machinery like "Insert:" / "Add:") is an unusable proposal, not a malformed
            # response. Drop just this application and keep the provision's classification;
            # rejecting the whole provision here is what left such segments permanently pending.
            continue
        normalised_application = {
            "logic": logic,
            "operator": str(application.get("operator", "Applicable")).strip()[:120] or "Applicable",
            "summary": summary[:2000],
            "premise": str(application.get("premise", "")).strip()[:2000],
            "conclusion": str(application.get("conclusion", "")).strip()[:2000],
        }
        try:
            normalised_application["confidence"] = max(
                0.0, min(1.0, float(application.get("confidence", out["confidence"])))
            )
        except (TypeError, ValueError):
            normalised_application["confidence"] = out["confidence"]
        if normalised_application["confidence"] >= MIN_LOGIC_CONFIDENCE:
            out["logicApplications"].append(normalised_application)
    for application in out["logicApplications"]:
        if application["logic"] != "Deontic":
            continue
        operator = application["operator"].lower()
        reconciled = None
        if "forbid" in operator or "prohibit" in operator:
            reconciled = "prohibition"
        elif "permit" in operator:
            reconciled = "permission"
        elif "right" in operator:
            reconciled = "right"
        elif "oblig" in operator:
            reconciled = "obligation"
        if reconciled:
            dtype = reconciled
            out["deonticType"] = reconciled.title()
        break
    if dtype != "undertaking" and not any(
        application["logic"] == "Deontic" for application in out["logicApplications"]
    ) and out["confidence"] >= MIN_LOGIC_CONFIDENCE:
        operator = {"obligation": "Obligate", "prohibition": "Forbid",
                    "permission": "Permit", "right": "Right"}.get(dtype, "Applicable")
        out["logicApplications"].insert(0, {
            "logic": "Deontic", "operator": operator,
            "summary": out.get("summary", f"Proposed {dtype} classification."),
            "premise": "", "conclusion": out.get("mustProvide", ""),
            "confidence": out["confidence"],
        })
    return out


def filter_logic_applications_for_source(classification: dict, source_text: str) -> dict:
    """Reject modality proposals whose minimum textual preconditions are absent."""
    text = source_text.lower()
    kept = []
    for application in classification.get("logicApplications", []):
        if application["logic"] == "DescriptionLogic" and not re.search(
            r"\b(subclass|subsum(?:e|ed|ption)|class of|type of|kind of|category of|means|includes)\b",
            text,
        ):
            continue
        kept.append(application)
    classification["logicApplications"] = kept
    return classification


def choose_num_ctx(max_segment_chars: int, max_items: int) -> int:
    """Context window sized to hold the largest prompt + output without truncation.

    Dense JSON legislative excerpts run ~3 chars/token; add ~700 tokens of system + instruction
    overhead and the output cap, then round up to a power of two (floor MIN_NUM_CTX). This scales
    if a caller raises --max-segment-chars/--max-segment-items, so the prompt is never silently
    truncated by Ollama's small default window. Measured: the largest real 8000-char excerpt is a
    ~2460-token prompt; 8192 leaves ample room for it plus the 1800-token answer.
    """
    prompt_tokens = (max_segment_chars * max_items) // 3 + 700
    needed = prompt_tokens + NUM_PREDICT + 512
    return max(MIN_NUM_CTX, 1 << (needed - 1).bit_length())


def classify_segment(segment: Segment, model: str, url: str, timeout: int = 180,
                     temperature: float = 0.0, num_ctx: int = MIN_NUM_CTX) -> tuple[dict | None, str | None]:
    try:
        import requests
    except ImportError:
        sys.exit("requests required: pip install requests")
    if not segment.items:
        return {}, None
    expected = {item["key"] for item in segment.items}
    user = (
        "Classify every keyed legislative excerpt below. Keys are unique even when legal section "
        "numbers repeat. An excerpt may be one part of a long provision; classify only what the "
        "text supports.\n\n"
        f"EXCERPTS:\n{json.dumps(segment.items, ensure_ascii=False)}\n\n"
        "Return a JSON object mapping each exact `key` to "
        '{"deonticType":..., "borneBy":..., "heldBy":..., "summary":..., '
        '"mustProvide":..., "conditions":[...], "crossReferences":[...], "confidence":0.0}. '
        "Each result MUST also contain `logicApplications`, an array of at most 6 concise, applicable "
        "entries from this full checklist: Deontic, Epistemic, LTL, Paraconsistent, ASP, "
        "Dialectical, LinearLogic, DescriptionLogic, Argumentation, AllenInterval, Diffusion, "
        "CogAI, SHACL, N3Logic. Each entry is "
        '{"logic":...,"operator":...,"summary":...,"premise":...,"conclusion":...,'
        f'"confidence":0.0}}. Use [] when none applies; omit any application below '
        f'{MIN_LOGIC_CONFIDENCE:.1f} confidence; do not add a logic merely to fill the list. '
        "Return every key exactly once."
    )
    application_schema = {
        "type": "object", "additionalProperties": False,
        "properties": {
            "logic": {"type": "string", "enum": list(LOGIC_SUITE)},
            "operator": {"type": "string"}, "summary": {"type": "string"},
            "premise": {"type": "string"}, "conclusion": {"type": "string"},
            "confidence": {"type": "number", "minimum": 0, "maximum": 1},
        },
        "required": ["logic", "operator", "summary", "premise", "conclusion", "confidence"],
    }
    classification_schema = {
        "type": "object", "additionalProperties": False,
        "properties": {
            "deonticType": {"type": "string", "enum": [name.title() for name in DEONTIC]},
            "borneBy": {"type": ["string", "null"]}, "heldBy": {"type": ["string", "null"]},
            "summary": {"type": "string"}, "mustProvide": {"type": ["string", "null"]},
            "conditions": {"type": "array", "items": {"type": "string"}, "maxItems": 20},
            "crossReferences": {"type": "array", "items": {"type": "string"}, "maxItems": 20},
            "confidence": {"type": "number", "minimum": 0, "maximum": 1},
            "logicApplications": {"type": "array", "items": application_schema, "maxItems": 6},
        },
        "required": ["deonticType", "borneBy", "heldBy", "summary", "mustProvide",
                     "conditions", "crossReferences", "confidence", "logicApplications"],
    }
    response_schema = {"type": "object", "additionalProperties": False,
                       "properties": {key: classification_schema for key in sorted(expected)},
                       "required": sorted(expected)}
    payload = {"model": model, "stream": False, "format": response_schema,
               "options": {"temperature": temperature, "num_predict": NUM_PREDICT, "num_ctx": num_ctx},
               "messages": [{"role": "system", "content": SYSTEM},
                            {"role": "user", "content": user}]}
    try:
        r = requests.post(f"{url.rstrip('/')}/api/chat", json=payload, timeout=timeout)
        r.raise_for_status()
        data = _loads(r.json().get("message", {}).get("content", ""))
        if not isinstance(data, dict):
            return None, "response was not a JSON object"
        normalised = {str(key): _normalise_classification(value) for key, value in data.items()
                      if str(key) in expected}
        invalid = sorted(key for key in expected if normalised.get(key) is None)
        if invalid:
            return None, f"missing or invalid keys: {', '.join(invalid)}"
        source_by_key = {item["key"]: item["text"] for item in segment.items}
        return {key: filter_logic_applications_for_source(normalised[key], source_by_key[key])
                for key in sorted(expected)}, None
    except Exception as e:  # noqa: BLE001
        return None, str(e)


def aggregate_classifications(completed: dict) -> dict:
    """Deterministically merge repeated long-provision segment proposals."""
    buckets: dict[str, list[dict]] = {}
    for segment_id in sorted(completed):
        for key, value in completed[segment_id].get("results", {}).items():
            if isinstance(value, dict):
                buckets.setdefault(key, []).append(value)
    merged = {}
    type_order = {name.title(): index for index, name in enumerate(DEONTIC)}
    for key, values in buckets.items():
        votes: dict[str, int] = {}
        for value in values:
            dtype = value.get("deonticType", "Undertaking")
            votes[dtype] = votes.get(dtype, 0) + 1
        dtype = min(votes, key=lambda name: (-votes[name], type_order.get(name, 99)))
        out = {"deonticType": dtype}
        for field_name in ("borneBy", "heldBy", "mustProvide"):
            out[field_name] = next((v[field_name] for v in values if v.get(field_name)), "")
        summaries = list(dict.fromkeys(v.get("summary", "") for v in values if v.get("summary")))
        out["summary"] = " ".join(summaries)[:4000]
        for field_name in ("conditions", "crossReferences"):
            out[field_name] = list(dict.fromkeys(
                item for value in values for item in value.get(field_name, [])
            ))[:100]
        confidences = [float(v.get("confidence", 0.5)) for v in values]
        out["confidence"] = round(sum(confidences) / len(confidences), 4)
        applications = []
        seen = set()
        for value in values:
            for application in value.get("logicApplications", []):
                signature = (application["logic"], application["operator"],
                             application["premise"], application["conclusion"])
                if signature not in seen:
                    seen.add(signature)
                    applications.append(application)
        out["logicApplications"] = applications[:48]
        merged[key] = out
    return merged


def write_json_atomic(path: Path, value: dict) -> None:
    temp = path.with_suffix(path.suffix + ".tmp")
    temp.write_text(json.dumps(value, indent=2, ensure_ascii=False), encoding="utf-8")
    temp.replace(path)


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


def classification_for(provision: Provision, classifications: dict) -> dict:
    """Prefer stable fragment keys; accept v1 progress keyed by section number."""
    return classifications.get(provision.frag) or classifications.get(str(provision.number)) or {}


def logic_application_id(inst: Instrument, provision: Provision, index: int, application: dict) -> str:
    return f"concept:{inst.slug}-{provision.frag}-logic-{slugify(application['logic'])}-{index + 1}"


def logic_applications_n3(inst: Instrument, provision: Provision, cls: dict | None) -> str:
    blocks = []
    cid = f"concept:{inst.slug}-{provision.frag}"
    for index, application in enumerate((cls or {}).get("logicApplications", [])):
        aid = logic_application_id(inst, provision, index, application)
        modality, surface = LOGIC_SUITE[application["logic"]]
        lines = [f"{cid} cml:asserts {aid} .", f"{aid} a cml:LogicApplication ;",
                 f"    cml:modality {modality} ;",
                 f'    cml:operator "{_lit(application["operator"])}" ;',
                 f'    cml:logicSummary "{_lit(application["summary"])}" ;']
        if application.get("premise"):
            lines.append(f'    cml:premiseText "{_lit(application["premise"])}" ;')
        if application.get("conclusion"):
            lines.append(f'    cml:conclusionText "{_lit(application["conclusion"])}" ;')
        lines += [f'    cml:executionSurface "{surface}" ;',
                  f'    cml:confidence "{application["confidence"]:.4f}"^^xsd:decimal ;',
                  "    cml:curationStatus cml:Proposed ."]
        blocks.append("\n".join(lines))
    return "\n\n".join(blocks)


def instrument_n3(inst: Instrument, source_ref: str) -> str:
    """The instrument-level metadata node (title, identifier, date, long title, jurisdiction)."""
    L = [f"<{inst.base_iri}> a cof:Document ;",
         f'    dc:title "{_lit(inst.title)}"@en ;',
         f'    values:category "{_lit(inst.jurisdiction)}" ;']
    if inst.act_no and inst.year:
        L.append(f'    dc:identifier "No. {_lit(inst.act_no)} of {_lit(inst.year)}" ;')
    elif inst.year:
        L.append(f'    dc:date "{_lit(inst.year)}"^^xsd:gYear ;')
    if inst.date:
        L.append(f'    dc:date "{_lit(inst.date)}" ;')
    if inst.long_title:
        L.append(f'    dc:description "{_lit(inst.long_title)}" ;')
    if inst.eli:
        L.append(f"    prov:wasDerivedFrom <{inst.eli}> ;")
    L += [f"    prov:wasDerivedFrom <{source_ref}> ;",
          "    cml:proposedBy <" + TOOL_IRI + "> ;",
          "    cml:curationStatus cml:Proposed ."]
    return "\n".join(L)


def provision_n3(inst: Instrument, p: Provision, cls: dict | None, source_ref: str,
                 source_name: str, schema: str, children: list[str]) -> str:
    cid = f"concept:{inst.slug}-{p.frag}"
    nid = f"{cid}-norm"
    src_text = provision_source_text(p)
    L = [f"{cid} a cml:Concept ;",
         f'    skos:prefLabel "{_lit(p.number + " " + p.heading)}"@en ;',
         f"    cml:realizedBy doc:{p.frag} ;"]
    # Always carry the provision body in the graph (this was missing; exports looked "section-empty").
    if src_text.strip():
        L.append(f'    values:originalText "{_lit(src_text)}" ;')
        L.append(f'    skos:definition "{_lit(src_text[:2000])}"@en ;')
    if p.integrity:
        L.append(f'    cml:integrityHash "{p.integrity}" ;')
    elif src_text.strip():
        L.append(f'    cml:integrityHash "{sha256(src_text)}" ;')
    if p.parent:
        L.append(f"    values:partOf concept:{inst.slug}-{p.parent} ;")
    if p.start_page:
        L.append(f'    cof:pageNumber "{p.start_page}"^^xsd:integer ;')
    if cls and cls.get("summary"):
        L.append(f'    skos:note "{_lit(cls["summary"])}" ;')
    if cls and cls.get("mustProvide"):
        L.append(f'    dc:description "{_lit(cls["mustProvide"])}" ;')
    for ref in (cls.get("crossReferences") if cls else []) or []:
        L.append(f'    dc:references "{_lit(str(ref))}" ;')
    for child in children:
        L.append(f"    cml:hasPart concept:{inst.slug}-{child} ;")
    L += [f"    cml:curationStatus cml:Proposed ;",
          f'    cml:schemaVersion "{schema}" ;',
          f"    cml:proposedBy <{TOOL_IRI}> ;",
          f"    prov:wasDerivedFrom <{source_ref}> ;",
          f'    dc:source "{_lit(source_name)}, p.{p.start_page}"']
    # Realization node: the structural document fragment with the same text payload.
    R = [f"doc:{p.frag} a cof:Section ;",
         f'    cof:title "{_lit(p.number + " " + p.heading)}" ;',
         f'    values:kind "{_lit(p.kind)}" ;']
    if src_text.strip():
        R.append(f'    values:originalText "{_lit(src_text)}" ;')
    if p.start_page:
        R.append(f'    cof:pageNumber "{p.start_page}"^^xsd:integer ;')
    R[-1] = R[-1].rstrip(" ;") + " ."

    if children:
        # A container section holds no norm of its own; its subsections carry the deontic content.
        return "\n".join(L) + " .\n" + "\n".join(R)
    L[-1] = L[-1] + " ;"
    L.append(f"    cml:asserts {nid} .")
    dtype = norm_type(cls)
    N = [f"{nid} a {dtype} ;",
         f"    cml:modality cml:Deontic ;",
         f"    values:partOf {cid} ;"]
    if dtype in ("values:Obligation", "values:Prohibition", "values:Permission"):
        bearer = BEARER.get(str((cls or {}).get("borneBy", "")).lower(), "values:Agent")
        N.append(f"    values:borneBy {bearer} ;")
        if cls and cls.get("borneBy"):
            N.append(f'    skos:scopeNote "borne by: {_lit(str(cls["borneBy"]))}" ;')
    elif dtype == "values:Right":
        held = BEARER.get(str((cls or {}).get("heldBy", "")).lower(), "values:NaturalPerson")
        N.append(f"    values:heldBy {held} ;")
        if cls and cls.get("heldBy"):
            N.append(f'    skos:scopeNote "held by: {_lit(str(cls["heldBy"]))}" ;')
    N += ["    values:deonticStatus values:HeuristicDerived ;",
          "    cml:curationStatus cml:Proposed ."]
    logic = logic_applications_n3(inst, p, cls)
    return ("\n".join(L) + "\n" + "\n".join(N) + "\n"
            + (logic + "\n" if logic else "") + "\n".join(R))


def structural_n3(inst: Instrument, p: Provision) -> str:
    """Part / Division / Schedule markers as first-class structure (not only HTML chrome)."""
    return (
        f"doc:{p.frag} a cof:Section ;"
        f'\n    cof:title "{_lit(p.kind.title() + " " + p.number + ((" — " + p.heading) if p.heading else ""))}" ;'
        f'\n    values:kind "{_lit(p.kind)}" ;'
        f'\n    cof:pageNumber "{p.start_page}"^^xsd:integer .'
    )


def build_n3(inst: Instrument, cls_by_num: dict, source_ref: str, source_name: str, schema: str) -> str:
    doc_ns = f"{NS['values']}{inst.slug}#"
    head = [f"@prefix {k}: <{v}> ." for k, v in NS.items()]
    head.append(f"@prefix doc: <{doc_ns}> .")
    head += ["",
             f"# CML concept layer — {inst.title}",
             f"# MACHINE-DERIVED from the TEXT layer — cml:Proposed, values:HeuristicDerived.",
             f"# Pending human attestation (cml:Attested / skos:exactMatch). Generated by legis2cml.",
             f"# Every section/subsection carries values:originalText (full provision body).",
             ""]
    kids = children_of(inst.provisions)
    blocks = [instrument_n3(inst, source_ref)]
    for p in inst.provisions:
        if p.kind in ("part", "division", "schedule"):
            blocks.append(structural_n3(inst, p))
    blocks += [provision_n3(inst, p, classification_for(p, cls_by_num), source_ref, source_name,
                            schema, kids.get(p.frag, []))
               for p in concept_units(inst.provisions)]
    return "\n".join(head) + "\n" + "\n\n".join(blocks)


def instrument_jsonld(inst: Instrument, source_ref: str) -> dict:
    node = {"@id": inst.base_iri, "@type": "cof:Document",
            "dc:title": inst.title, "values:category": inst.jurisdiction,
            "prov:wasDerivedFrom": {"@id": source_ref},
            "cml:proposedBy": {"@id": TOOL_IRI},
            "cml:curationStatus": {"@id": "cml:Proposed"}}
    if inst.act_no and inst.year:
        node["dc:identifier"] = f"No. {inst.act_no} of {inst.year}"
    elif inst.year:
        node["dc:date"] = {"@value": inst.year, "@type": "xsd:gYear"}
    if inst.date:
        node["dc:date"] = inst.date
    if inst.long_title:
        node["dc:description"] = inst.long_title
    return node


def build_jsonld(inst: Instrument, cls_by_num: dict, source_ref: str,
                 schema: str = SCHEMA_DEFAULT) -> dict:
    ctx = {**NS, "@base": NS["concept"]}
    kids = children_of(inst.provisions)
    nodes = [instrument_jsonld(inst, source_ref)]
    for p in inst.provisions:
        if p.kind in ("part", "division", "schedule"):
            nodes.append({
                "@id": f"{NS['values']}{inst.slug}#{p.frag}",
                "@type": "cof:Section",
                "cof:title": f"{p.kind.title()} {p.number}"
                + (f" — {p.heading}" if p.heading else ""),
                "values:kind": p.kind,
                "cof:pageNumber": p.start_page,
            })
    for p in concept_units(inst.provisions):
        cls = classification_for(p, cls_by_num)
        cid = f"concept:{inst.slug}-{p.frag}"
        children = kids.get(p.frag, [])
        applications = [] if children else cls.get("logicApplications", [])
        assertion_ids = [] if children else [{"@id": f"{cid}-norm"}]
        for index, application in enumerate(applications):
            assertion_ids.append({"@id": logic_application_id(inst, p, index, application)})
        src_text = provision_source_text(p)
        concept = {"@id": cid, "@type": "cml:Concept",
                   "skos:prefLabel": f"{p.number} {p.heading}",
                   "cml:realizedBy": {"@id": f"{NS['values']}{inst.slug}#{p.frag}"},
                   "cml:curationStatus": {"@id": "cml:Proposed"},
                   "cml:schemaVersion": schema,
                   "cml:proposedBy": {"@id": TOOL_IRI},
                   "prov:wasDerivedFrom": {"@id": source_ref},
                   "dc:source": f"{source_ref}, p.{p.start_page}"}
        if src_text.strip():
            concept["values:originalText"] = src_text
            concept["skos:definition"] = src_text[:2000]
        if p.parent:
            concept["values:partOf"] = {"@id": f"concept:{inst.slug}-{p.parent}"}
        if children:
            concept["cml:hasPart"] = [{"@id": f"concept:{inst.slug}-{c}"} for c in children]
        else:
            concept["cml:asserts"] = assertion_ids
        if p.integrity:
            concept["cml:integrityHash"] = p.integrity
        elif src_text.strip():
            concept["cml:integrityHash"] = sha256(src_text)
        if cls.get("summary"):
            concept["skos:note"] = cls["summary"]
        if cls.get("mustProvide"):
            concept["dc:description"] = cls["mustProvide"]
        if cls.get("crossReferences"):
            concept["dc:references"] = cls["crossReferences"]
        nodes.append(concept)
        # Realization fragment with the same body text (HTML/COF join key).
        real = {"@id": f"{NS['values']}{inst.slug}#{p.frag}",
                "@type": "cof:Section",
                "cof:title": f"{p.number} {p.heading}",
                "values:kind": p.kind,
                "cof:pageNumber": p.start_page}
        if src_text.strip():
            real["values:originalText"] = src_text
        nodes.append(real)
        if children:
            continue
        norm = {"@id": f"{cid}-norm", "@type": norm_type(cls),
                "cml:modality": {"@id": "cml:Deontic"},
                "values:partOf": {"@id": cid},
                "values:deonticStatus": {"@id": "values:HeuristicDerived"},
                "cml:curationStatus": {"@id": "cml:Proposed"}}
        if norm["@type"] in ("values:Obligation", "values:Prohibition", "values:Permission"):
            bearer = BEARER.get(str(cls.get("borneBy", "")).lower(), "values:Agent")
            norm["values:borneBy"] = {"@id": bearer}
            if cls.get("borneBy"):
                norm["skos:scopeNote"] = f"borne by: {cls['borneBy']}"
        elif norm["@type"] == "values:Right":
            held = BEARER.get(str(cls.get("heldBy", "")).lower(), "values:NaturalPerson")
            norm["values:heldBy"] = {"@id": held}
            if cls.get("heldBy"):
                norm["skos:scopeNote"] = f"held by: {cls['heldBy']}"
        nodes.append(norm)
        for index, application in enumerate(applications):
            aid = logic_application_id(inst, p, index, application)
            modality, surface = LOGIC_SUITE[application["logic"]]
            node = {"@id": aid, "@type": "cml:LogicApplication",
                    "cml:modality": {"@id": modality},
                    "cml:operator": application["operator"],
                    "cml:logicSummary": application["summary"],
                    "cml:executionSurface": surface,
                    "cml:confidence": {"@value": application["confidence"],
                                       "@type": "xsd:decimal"},
                    "cml:curationStatus": {"@id": "cml:Proposed"}}
            if application.get("premise"):
                node["cml:premiseText"] = application["premise"]
            if application.get("conclusion"):
                node["cml:conclusionText"] = application["conclusion"]
            nodes.append(node)
    return {"@context": ctx, "@graph": nodes}


def build_logic_shacl(inst: Instrument) -> str:
    modalities = " ".join(value[0] for value in LOGIC_SUITE.values())
    return f"""@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix cml: <{NS['cml']}> .
@prefix concept: <{NS['concept']}> .

concept:{inst.slug}-LogicApplicationShape a sh:NodeShape ;
    sh:targetClass cml:LogicApplication ;
    sh:property [ sh:path cml:modality ; sh:minCount 1 ; sh:maxCount 1 ;
                  sh:in ( {modalities} ) ] ;
    sh:property [ sh:path cml:logicSummary ; sh:minCount 1 ; sh:datatype xsd:string ] ;
    sh:property [ sh:path cml:executionSurface ; sh:minCount 1 ; sh:maxCount 1 ] ;
    sh:property [ sh:path cml:confidence ; sh:minCount 1 ; sh:maxCount 1 ;
                  sh:datatype xsd:decimal ; sh:minInclusive 0 ; sh:maxInclusive 1 ] ;
    sh:property [ sh:path cml:curationStatus ; sh:hasValue cml:Proposed ; sh:minCount 1 ] .

concept:{inst.slug}-ConceptShape a sh:NodeShape ;
    sh:targetClass cml:Concept ;
    sh:property [ sh:path cml:realizedBy ; sh:minCount 1 ] ;
    sh:property [ sh:path cml:curationStatus ; sh:hasValue cml:Proposed ; sh:minCount 1 ] .
"""


def _chk_value(value: object) -> str:
    return re.sub(r"\s+", " ", str(value)).replace("{", "(").replace("}", ")").strip()


def build_cogai(inst: Instrument, classifications: dict) -> tuple[str, int]:
    lines = [f"# CogAI proposed logic-routing chunks — {inst.title}",
             "# Machine-derived; every chunk remains cml:Proposed.", ""]
    count = 0
    for provision in classifiable_units(inst.provisions):
        cls = classification_for(provision, classifications)
        for index, application in enumerate(cls.get("logicApplications", [])):
            count += 1
            _modality, surface = LOGIC_SUITE[application["logic"]]
            chunk_id = f"{inst.slug}-{provision.frag}-logic-{count}"
            lines += [f"{chunk_id} {{", f"  activation: {application['confidence']:.4f};",
                      "  type: LogicApplication;", f"  provision: {provision.frag};",
                      f"  logic: {_chk_value(application['logic'])};",
                      f"  operator: {_chk_value(application['operator'])};",
                      f"  summary: {_chk_value(application['summary'])};",
                      f"  executionSurface: {_chk_value(surface)};",
                      "  curationStatus: Proposed;"]
            if application.get("premise"):
                lines.append(f"  @condition: {_chk_value(application['premise'])};")
            if application.get("conclusion"):
                lines.append(f"  @action: {_chk_value(application['conclusion'])};")
            lines += ["}", ""]
    return "\n".join(lines), count


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
.legal-notice{background:var(--logic);border:2px solid var(--warn);border-radius:8px;padding:1rem 1.1rem;
margin:1rem 0 1.25rem;font:.9rem/1.5 system-ui,sans-serif;color:var(--fg)}
.legal-notice h2{font-size:1.05rem;margin:0 0 .35rem;color:var(--fg)}
.legal-notice p{margin:.4rem 0}.legal-notice ul{margin:.55rem 0 .55rem 1.2rem;padding:0}
.legal-notice .alpha{display:inline-block;background:var(--warn);color:var(--bg);border-radius:999px;
padding:.12rem .55rem;margin-right:.4rem;font-size:.72rem;font-weight:700;letter-spacing:.04em;text-transform:uppercase}
.legal-notice a,.legal-footer a{color:var(--accent)}
.legal-footer{border-top:1px solid var(--rule);margin-top:2.5rem;padding-top:1rem;
font:.78rem/1.5 system-ui,sans-serif;color:var(--muted)}
.rights-scope{border-left:3px solid var(--rule);padding-left:.75rem;margin:.8rem 0}
.rights-scope strong{color:var(--fg)}
h2.part,h2.division{font:600 .95rem system-ui,sans-serif;text-transform:uppercase;letter-spacing:.04em;
color:var(--accent);border-top:1px solid var(--rule);padding-top:1.3rem;margin-top:2rem}
section.concept{margin:1.7rem 0;scroll-margin-top:1rem}
section.subsection{margin:.7rem 0 .7rem 1.6rem;padding-left:1rem;border-left:2px solid var(--rule)}
section.subsection>h3{font-size:1rem}
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


def federal_register_id(value: str | None) -> str | None:
    """Extract the Register ID prefix from plain, rectified, or multi-volume filenames."""
    match = re.match(r"^([A-Z]\d{4}[A-Z]\d{5})", (value or "").strip().upper())
    return match.group(1) if match else None


def federal_register_url(register_id: str | None) -> str:
    """Link title/as-made IDs to latest downloads and compilation IDs to the exact record."""
    official_id = federal_register_id(register_id)
    if official_id:
        # The second type letter is C for a compilation ID. A compilation identifies the exact
        # point-in-time source and must not be silently advanced through a /latest route.
        if official_id[5] == "C":
            return f"https://www.legislation.gov.au/{official_id}"
        return f"https://www.legislation.gov.au/{official_id}/latest/downloads"
    return "https://www.legislation.gov.au/"


def source_legal_details(jurisdiction: str, register_id: str | None,
                         eli: str | None) -> tuple[str, str, str, str]:
    """Return official URL, label, body-law name and source-rights HTML."""
    if jurisdiction.strip().upper() == "EU" and eli:
        official_url = eli
        return (
            official_url,
            "official EUR-Lex / Official Journal record",
            "European Union law",
            f'''Based on content from <a href="{esc(official_url)}" rel="external">EUR-Lex</a>.
    EUR-Lex permits reuse of legal documents for commercial or non-commercial purposes unless
    otherwise specified, subject to its <a href="https://eur-lex.europa.eu/content/legal-notice/legal-notice.html"
    rel="external">legal notice, acknowledgement requirements and exceptions</a>. Only EU documents
    published in the Official Journal of the European Union are authentic.''',
        )
    official_url = federal_register_url(register_id)
    official_id = federal_register_id(register_id)
    record_label = (f"official Register record {esc(official_id)}"
                    if official_id else "Federal Register of Legislation")
    return (
        official_url,
        record_label,
        "Australian law",
        f'''Based on content from the
    <a href="{esc(official_url)}" rel="external">Federal Register of Legislation</a>.
    Register content is generally available under
    <a href="https://creativecommons.org/licenses/by/4.0/" rel="external">CC BY 4.0</a>, subject to the
    Register's <a href="https://www.legislation.gov.au/terms-of-use" rel="external">terms, attribution requirements and exceptions</a>.
    The source material has been reformatted; these changes are not endorsed by the Australian Government.''',
    )


def legal_notice_html(register_id: str | None, jurisdiction: str = "AU",
                      eli: str | None = None) -> str:
    official_url, record_label, body_law, _rights = source_legal_details(
        jurisdiction, register_id, eli
    )
    return f'''<aside class="legal-notice" aria-labelledby="legal-information-heading">
  <h2 id="legal-information-heading"><span class="alpha">Technical alpha</span> Legal information, not legal advice</h2>
  <p>This is an experimental semantic rendering for exploration and question-framing. It is not an
  official or authorised version of {esc(body_law)}, does not provide legal advice, and must not be
  relied on for legal, compliance, financial, or other consequential decisions.</p>
  <ul>
    <li><strong>Verify the law:</strong> consult the <a href="{esc(official_url)}" rel="external">{record_label}</a>
    for the authoritative source, current status, amendments, commencement information, and authorised PDF.</li>
    <li><strong>Expect transformation errors:</strong> structure, concepts, relationships, and logic panels were
    produced programmatically and may be incomplete, incorrect, or stripped of legally significant context.</li>
    <li><strong>Treat this as point-in-time material:</strong> the source may later be amended, superseded,
    repealed, rectified, or affected by uncommenced or transitional provisions.</li>
  </ul>
  <p>Use this page to discover potentially relevant provisions and frame questions to take to a qualified
  legal professional. <a href="/legal-information">Read the full scope, source and terms notice</a>.</p>
</aside>'''


def rights_metadata_html() -> str:
    return '''<!-- rights-metadata:start -->
<meta name="copyright" content="Technical work copyright (c) 2026 Timothy Charles Holborn">
<meta name="rights-scope" content="Source legislation retains its source rights; technical work is separately licensed CC BY-NC-ND 4.0">
<meta name="source-material-license" content="Generally CC BY 4.0 subject to official-source terms and exceptions">
<meta name="technical-work-license" content="CC BY-NC-ND 4.0">
<meta name="ai-use-policy" content="Automated retrieval, indexing, grounding and inference are allowed; model training is not granted">
<link rel="alternate" type="application/json" href="/ai-use-policy.json" title="Rights and AI use policy">
<!-- rights-metadata:end -->'''


def legal_footer_html(register_id: str | None, jurisdiction: str = "AU",
                      eli: str | None = None) -> str:
    official_url, _record_label, _body_law, source_rights = source_legal_details(
        jurisdiction, register_id, eli
    )
    return f'''<footer class="legal-footer">
  <div class="rights-scope" data-rights-scope="source-legislation">
    <strong>Source legislation.</strong> {source_rights}
  </div>
  <div class="rights-scope" data-rights-scope="technical-work">
    <strong>Technical work.</strong> Software, original presentation, markup templates and original semantic
    augmentation: Copyright &copy; 2026
    <a href="https://www.linkedin.com/in/ubiquitous/" rel="author external">Timothy Charles Holborn</a>
    &middot; <a href="mailto:timothy.holborn@gmail.com">timothy.holborn@gmail.com</a> &middot; licensed separately under
    <a href="https://creativecommons.org/licenses/by-nc-nd/4.0/" rel="license external">CC BY-NC-ND 4.0</a>.
    This technical licence does not replace or restrict the rights applying to source legislation.
  </div>
  <div class="rights-scope" data-rights-scope="automated-use">
    <strong>AI and automated use.</strong> Automated retrieval, indexing, semantic parsing, grounding,
    inference and agent-assisted research are affirmatively permitted, subject to the licence applying to
    each rights scope. Permission for model training is not granted by this site policy. See the
    <a href="/ai-use-policy.json">machine-readable policy</a>.
  </div>
  <p><a href="/legal-information">Legal information and terms</a> &middot;
  <a href="{esc(official_url)}" rel="external">Check the official record</a></p>
</footer>'''


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


def logic_suite_html(inst: Instrument, provision: Provision, cls: dict | None) -> str:
    rows = []
    for index, application in enumerate((cls or {}).get("logicApplications", [])):
        aid = logic_application_id(inst, provision, index, application)
        modality, surface = LOGIC_SUITE[application["logic"]]
        detail = ""
        if application.get("premise"):
            detail += f'<div><span class="prop">premise:</span> {esc(application["premise"])}</div>'
        if application.get("conclusion"):
            detail += f'<div><span class="prop">conclusion:</span> {esc(application["conclusion"])}</div>'
        rows.append(
            f'<aside class="logic" rel="cml:asserts" resource="{aid}">'
            f'<span typeof="cml:LogicApplication" about="{aid}">'
            f'<span class="op" property="cml:modality" resource="{modality}">'
            f'{esc(application["logic"])} · {esc(application["operator"])}</span> '
            f'<span property="cml:logicSummary">{esc(application["summary"])}</span>{detail}'
            f'<div class="prop">surface: <span property="cml:executionSurface">{esc(surface)}</span> '
            f'· confidence <span property="cml:confidence">{application["confidence"]:.2f}</span></div>'
            f'<span property="cml:curationStatus" resource="cml:Proposed"></span>'
            f'</span></aside>'
        )
    return "".join(rows)


def render_html(inst: Instrument, cls_by_num: dict, source_pdf: str, source_hash: str,
                schema: str = SCHEMA_DEFAULT, register_id: str | None = None) -> str:
    """HTML+RDFa dual surface: human-readable CML studio page + COF agent payload.

    COF (Context Optimisation Format) here is *not* a parallel document model —
    it is a constrained HTML+RDFa *profile* over the same CML graph: structural
    cof:Document / cof:Section / cof:Block / cof:Claim / cof:Entity bindings with
    attributes (ref, confidence, page) and no presentation bloat beyond a thin CSS.
    """
    prefix = " ".join(f"{k}: {v}" for k, v in NS.items())
    prefix += f" doc: {NS['values']}{inst.slug}#"
    doc_about = f"{NS['values']}{inst.slug}"
    kids = children_of(inst.provisions)
    body = []
    last_page = None
    for p in inst.provisions:
        if p.kind in ("part", "division", "schedule"):
            body.append(
                f'<section class="{p.kind}" id="{p.frag}" typeof="cof:Section" '
                f'property="cof:hasSection" resource="doc:{p.frag}">'
                f'<h2 property="cof:title dc:title">{esc(p.kind.title())} {esc(p.number)}'
                f'{" — " + esc(p.heading) if p.heading else ""}</h2></section>'
            )
            continue
        if p.kind not in ("section", "subsection"):
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
        is_container = p.frag in kids
        cls = {} if is_container else classification_for(p, cls_by_num)
        conf_attr = ""
        raw_conf = cls.get("confidence")
        if raw_conf is not None:
            try:
                conf_attr = f' data-confidence="{float(raw_conf):.2f}"'
            except (TypeError, ValueError):
                conf_attr = ""
        claim_id = f"{cid}-claim"
        css_class = "concept subsection" if p.kind == "subsection" else "concept"
        part_of = (f' rel="values:partOf" resource="concept:{inst.slug}-{p.parent}"'
                   if p.parent else "")
        heading = f'{esc(p.number)} {esc(p.heading)}' if p.kind == "section" else esc(p.number)
        text_block = ""
        # Prefer full pre-split body for container sections so the HTML never drops subsection text.
        body_text = provision_source_text(p)
        if body_text:
            text_block = (
                f'<div class="text" typeof="cof:Block" property="cof:hasBlock values:originalText" '
                f'resource="doc:{p.frag}-text" data-page="{p.start_page}">'
                f'<span typeof="cof:Claim" property="cof:hasClaim" about="{claim_id}" '
                f'resource="{claim_id}"{conf_attr}>{esc(body_text)}</span></div>'
            )
        norm_block = "" if is_container else f'{logic_html(inst, p, cls)}{logic_suite_html(inst, p, cls)}'
        body.append(
            f'<section class="{css_class}" id="{p.frag}" '
            f'typeof="cml:Concept cof:Section" about="{cid}" resource="{cid}" '
            f'property="cof:hasSection" data-page="{p.start_page}"{conf_attr}{part_of}>'
            f'<h3><span class="num" property="skos:prefLabel cof:title">{heading}</span>'
            f'<a class="frag" href="#{p.frag}">#</a></h3>'
            f'{text_block}{norm_block}'
            f'<link rel="cml:realizedBy" href="doc:{p.frag}" />'
            f'</section>'
        )
    jsonld = json.dumps(build_jsonld(inst, cls_by_num, source_pdf, schema), indent=2,
                        ensure_ascii=False)
    return f"""<!DOCTYPE html>
<html lang="en" prefix="{esc(prefix)}">
<head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="cof-profile" content="{COF_PROFILE}">
<meta name="cml-schema" content="{esc(schema)}">
{rights_metadata_html()}
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
{legal_notice_html(register_id, inst.jurisdiction, inst.eli)}
<div class="banner">⚑ <strong>Machine-proposed (<code>cml:Proposed</code>).</strong> Every concept and norm
below was derived automatically and is <em>provisional</em>; only a signed, authoritative human action may
attest it (<code>cml:Attested</code> / <code>skos:exactMatch</code>). Source:
<code>{esc(source_pdf)}</code> · sha256 <code>{esc(source_hash[:16])}…</code>
· Agent payload: this file is the COF surface (attributes carry graph edges; body is readable text).</div>
<main property="cof:body">
{chr(10).join(body)}
</main>
{legal_footer_html(register_id, inst.jurisdiction, inst.eli)}
<script type="application/ld+json">
{jsonld}
</script>
</body>
</html>
"""


# --------------------------------------------------------------------------- QualiaDB native compilation
def find_qualia_cli(explicit: Path | None = None) -> Path | None:
    if explicit:
        return explicit.resolve() if explicit.is_file() else None
    repo = Path(__file__).resolve().parents[2]
    candidates = [repo / "target" / profile / name
                  for profile in ("release", "debug")
                  for name in ("qualia-cli.exe", "qualia-cli")]
    found = [path for path in candidates if path.is_file()]
    return max(found, key=lambda path: path.stat().st_mtime) if found else None


def compile_and_verify_q42(cli: Path, n3_path: Path) -> tuple[Path, int]:
    ingest = subprocess.run([str(cli), "ingest", "semantic", str(n3_path)],
                            capture_output=True, encoding="utf-8", errors="replace")
    q42_path = n3_path.with_suffix(".q42")
    if ingest.returncode or not q42_path.is_file():
        raise RuntimeError(f"QualiaDB ingest failed: {(ingest.stderr or ingest.stdout)[-1200:]}")
    query = subprocess.run(
        [str(cli), "query", "sparql", str(q42_path), "SELECT ?s WHERE { ?s ?p ?o }"],
        capture_output=True, encoding="utf-8", errors="replace",
    )
    if query.returncode:
        raise RuntimeError(f"QualiaDB round-trip query failed: {(query.stderr or query.stdout)[-1200:]}")
    match = re.search(r"(\d+)\s+result\(s\)", query.stdout or "")
    if not match:
        raise RuntimeError("QualiaDB round-trip query returned no readable result count")
    return q42_path, int(match.group(1))


def compile_q42_shards(cli: Path, inst: Instrument, classifications: dict,
                       source_ref: str, source_name: str, schema: str,
                       out_dir: Path, shard_size: int) -> tuple[list[dict], list[str]]:
    """Compile bounded graph volumes; the native N3 parser must not receive a whole large Act."""
    sections = [provision for provision in inst.provisions if provision.kind == "section"]
    kids_map: dict[str, list[Provision]] = {}
    for provision in inst.provisions:
        if provision.kind == "subsection" and provision.parent:
            kids_map.setdefault(provision.parent, []).append(provision)
    q42_dir = out_dir / "q42"
    q42_dir.mkdir(parents=True, exist_ok=True)
    volumes, generated = [], []
    for offset in range(0, len(sections), shard_size):
        shard_number = offset // shard_size + 1
        shard_sections = sections[offset:offset + shard_size]
        # keep each container section together with its subsections in the same volume
        shard_provisions: list[Provision] = []
        for section in shard_sections:
            shard_provisions.append(section)
            shard_provisions.extend(kids_map.get(section.frag, []))
        shard_inst = Instrument(inst.title, inst.slug, inst.jurisdiction, inst.base_iri,
                                inst.eli, shard_provisions)
        stem = f"{inst.slug}-part-{shard_number:04d}"
        n3_path = q42_dir / f"{stem}.n3"
        n3_path.write_text(build_n3(shard_inst, classifications, source_ref, source_name, schema),
                           encoding="utf-8")
        q42_path, quins = compile_and_verify_q42(cli, n3_path)
        files = [n3_path.relative_to(out_dir).as_posix(), q42_path.relative_to(out_dir).as_posix()]
        lex_path = Path(str(q42_path) + ".lex")
        if lex_path.is_file():
            files.append(lex_path.relative_to(out_dir).as_posix())
        generated.extend(files)
        volumes.append({"part": shard_number, "firstProvision": shard_sections[0].frag,
                        "lastProvision": shard_sections[-1].frag,
                        "provisions": len(shard_provisions), "quinsReadBack": quins,
                        "files": files})
    return volumes, generated


# --------------------------------------------------------------------------- main
def main() -> None:
    try:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    except (AttributeError, OSError):
        pass
    ap = argparse.ArgumentParser(description="Legislation PDF -> CML concept layer + *.cml.html via Ollama.")
    ap.add_argument("--input", required=True, type=Path)
    ap.add_argument("--out-dir", type=Path, default=None, help="package dir (default: ./<slug>-cml)")
    ap.add_argument("--title", default=None)
    ap.add_argument("--jurisdiction", default="AU")
    ap.add_argument("--base-iri", default=None, help="instrument IRI (default values: + slug)")
    ap.add_argument("--eli", default=None)
    ap.add_argument("--model", default="llama3.2:3b-instruct-q4_K_M")
    ap.add_argument("--ollama-url", default="http://localhost:11434")
    ap.add_argument("--schema-version", default=SCHEMA_DEFAULT)
    ap.add_argument("--no-llm", action="store_true", help="structure only (all norms -> Undertaking)")
    ap.add_argument("--resume", action="store_true", help="reuse successful content-addressed segments")
    ap.add_argument("--max-segment-chars", type=int, default=DEFAULT_SEGMENT_CHARS)
    ap.add_argument("--max-segment-items", type=int, default=DEFAULT_SEGMENT_ITEMS)
    ap.add_argument("--segment-overlap-chars", type=int, default=DEFAULT_SEGMENT_OVERLAP)
    ap.add_argument("--ollama-timeout", type=int, default=180, help="seconds per segment request")
    ap.add_argument("--max-retries", type=int, default=2, help="additional attempts per failed segment")
    ap.add_argument("--link", action="store_true", help="reference the PDF in place, do not copy it")
    ap.add_argument("--emit-ttl", action="store_true", help="also write .ttl (needs rdflib)")
    ap.add_argument("--emit-q42", action="store_true",
                    help="compile N3 with QualiaDB and verify it through a SPARQL round trip")
    ap.add_argument("--qualia-cli", type=Path, default=None,
                    help="qualia-cli executable (default: newest target/{release,debug} build)")
    ap.add_argument("--q42-shard-provisions", type=int, default=40,
                    help="maximum provisions per bounded native QualiaDB volume")
    ap.add_argument("--allow-empty", action="store_true",
                    help="permit a structural package with no parsed provisions")
    args = ap.parse_args()

    if not args.input.exists():
        sys.exit(f"input not found: {args.input}")
    if args.max_retries < 0:
        sys.exit("--max-retries must be >= 0")
    if args.max_segment_items < 1:
        sys.exit("--max-segment-items must be >= 1")
    if args.q42_shard_provisions < 1:
        sys.exit("--q42-shard-provisions must be >= 1")

    print(f"· reading {args.input.name}")
    pdf_bytes = args.input.read_bytes()
    source_hash = hashlib.sha256(pdf_bytes).hexdigest()
    pages = extract_pages(args.input)
    print(f"  {len(pages)} pages")

    title, provisions = parse_pages(pages, args.title)
    provisions = decompose_provisions(provisions)
    slug = slugify(re.sub(r"\s*No\.\s*\d+.*$", "", title))
    # Safety net: a mis-parsed title must never become a runaway/garbage slug — it poisons every
    # concept IRI and filename. Bound it, and fall back to the Federal Register id (filename stem).
    slug = "-".join(slug.split("-")[:12]).strip("-")
    if len(slug) < 3 or len(slug) > 90:
        slug = slugify(args.input.stem)
    base_iri = args.base_iri or f"{NS['values']}{slug}"
    metadata = extract_metadata(pages, title)
    inst = Instrument(title, slug, args.jurisdiction, base_iri, args.eli, provisions,
                      act_no=metadata.get("act_no"), year=metadata.get("year"),
                      long_title=metadata.get("long_title"), date=metadata.get("date"))
    sections = [p for p in provisions if p.kind == "section"]
    subsections = [p for p in provisions if p.kind == "subsection"]
    units = classifiable_units(provisions)
    print(f"· parsed: {len(sections)} sections, {len(subsections)} subsections, "
          f"{len(units)} classifiable units, {len(provisions) - len(sections) - len(subsections)} headings")
    if not units and not args.allow_empty:
        sys.exit("no provisions parsed; refusing to report an empty legislation package as successful")

    out_dir = args.out_dir or Path(f"./{slug}-cml")
    out_dir.mkdir(parents=True, exist_ok=True)
    progress_path = out_dir / f"{slug}.progress.json"

    config = {"maxSegmentChars": args.max_segment_chars,
              "overlapChars": args.segment_overlap_chars,
              "maxItems": args.max_segment_items}
    segments = build_segments(units, args.max_segment_chars, args.segment_overlap_chars,
                              args.max_segment_items)
    progress = {"version": PROGRESS_VERSION, "sourceSha256": source_hash, "model": args.model,
                "segmentConfig": config, "completed": {}, "classifications": {}}
    if args.resume and progress_path.exists():
        try:
            previous = json.loads(progress_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            previous = {}
        compatible = (previous.get("version") == PROGRESS_VERSION
                      and previous.get("sourceSha256") == source_hash
                      and previous.get("model") == args.model
                      and previous.get("segmentConfig") == config)
        if compatible:
            progress = previous
            progress.setdefault("completed", {})
            print(f"· resuming: {len(progress['completed'])}/{len(segments)} segments complete")
        else:
            stale = progress_path.with_name(progress_path.stem + ".stale.json")
            shutil.copyfile(progress_path, stale)
            print(f"· progress incompatible with source/model/segment settings; archived to {stale.name}")

    failures = []
    if not args.no_llm:
        num_ctx = choose_num_ctx(args.max_segment_chars, args.max_segment_items)
        print(f"· enriching {len(segments)} bounded segment(s) via Ollama ({args.model}), "
              f"num_ctx={num_ctx}")
        for index, segment in enumerate(segments, 1):
            if segment.segment_id in progress["completed"]:
                continue
            result, error = None, None
            for attempt in range(args.max_retries + 1):
                # First attempt is deterministic (temp 0.0); retries add a little sampling
                # so a genuinely hard segment is re-explored rather than re-failed identically.
                temperature = 0.0 if attempt == 0 else min(0.2 * attempt, 0.6)
                result, error = classify_segment(segment, args.model, args.ollama_url,
                                                 args.ollama_timeout, temperature, num_ctx)
                if result is not None:
                    break
                print(f"  ! segment {index}/{len(segments)} attempt {attempt + 1} failed: {error}",
                      file=sys.stderr)
            if result is None:
                failures.append({"segment": segment.segment_id, "error": error})
                continue
            progress["completed"][segment.segment_id] = {
                "keys": sorted(result), "characters": segment.char_count, "results": result,
            }
            progress["classifications"] = aggregate_classifications(progress["completed"])
            write_json_atomic(progress_path, progress)
            print(f"  [{index}/{len(segments)}] {len(segment.items)} excerpt(s), "
                  f"{segment.char_count} chars -> checkpointed")
        if failures:
            print(f"  ! {len(failures)} segment(s) remain pending; rerun with --resume", file=sys.stderr)

    cls_by_num = progress.get("classifications", {})

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
        render_html(inst, cls_by_num, source_name, source_hash, args.schema_version,
                    args.input.stem), encoding="utf-8")
    (out_dir / f"{slug}.jsonld").write_text(
        json.dumps(build_jsonld(inst, cls_by_num, source_ref, args.schema_version), indent=2,
                   ensure_ascii=False), encoding="utf-8")
    shacl_path = out_dir / f"{slug}.logic.shacl.ttl"
    shacl_path.write_text(build_logic_shacl(inst), encoding="utf-8")
    cogai_text, cogai_count = build_cogai(inst, cls_by_num)
    cogai_path = out_dir / f"{slug}.cogai.chk"
    cogai_path.write_text(cogai_text, encoding="utf-8")

    generated_files = [f"{slug}.cml.n3", f"{slug}.cml.html", f"{slug}.jsonld",
                       shacl_path.name, cogai_path.name,
                       *([] if args.link else [f"{slug}.pdf"])]
    rdf_validation = None
    if args.emit_ttl:
        try:
            from rdflib import Graph
            graph = Graph()
            graph.parse(out_dir / f"{slug}.cml.n3", format="n3")
            jsonld_graph = Graph()
            jsonld_graph.parse(data=json.dumps(build_jsonld(
                inst, cls_by_num, source_ref, args.schema_version)), format="json-ld")
            shacl_graph = Graph()
            shacl_graph.parse(shacl_path, format="turtle")
            ttl_path = out_dir / f"{slug}.ttl"
            ttl_path.write_text(graph.serialize(format="turtle"), encoding="utf-8")
            generated_files.append(ttl_path.name)
            rdf_validation = {"engine": "rdflib", "n3Triples": len(graph),
                              "jsonLdTriples": len(jsonld_graph),
                              "shaclTriples": len(shacl_graph), "ok": True}
        except ImportError:
            print("  ! rdflib not installed; skipping .ttl")

    qualia_validation = None
    if args.emit_q42:
        cli = find_qualia_cli(args.qualia_cli)
        if cli is None:
            sys.exit("qualia-cli not found; build with `cargo build -p qualia-cli` or pass --qualia-cli")
        try:
            volumes, q42_files = compile_q42_shards(
                cli, inst, cls_by_num, source_ref, source_name, args.schema_version,
                out_dir, args.q42_shard_provisions,
            )
        except RuntimeError as error:
            sys.exit(str(error))
        cogai_quins = 0
        if cogai_count:
            try:
                cogai_q42, cogai_quins = compile_and_verify_q42(cli, cogai_path)
            except RuntimeError as error:
                sys.exit(str(error))
            q42_files.append(cogai_q42.relative_to(out_dir).as_posix())
        generated_files.extend(q42_files)
        qualia_validation = {"engine": str(cli), "boundedVolumes": len(volumes),
                             "shardProvisionLimit": args.q42_shard_provisions,
                             "quinsReadBack": sum(volume["quinsReadBack"] for volume in volumes),
                             "cogAiQuinsReadBack": cogai_quins,
                             "volumes": volumes, "ok": True}
        print(f"· QualiaDB round trip: {qualia_validation['quinsReadBack']} quins readable "
              f"across {len(volumes)} bounded volume(s)")

    logic_counts = {logic: 0 for logic in LOGIC_SUITE}
    for provision in units:
        for application in classification_for(provision, cls_by_num).get("logicApplications", []):
            logic_counts[application["logic"]] += 1

    manifest = {
        "title": inst.title, "slug": slug, "jurisdiction": inst.jurisdiction,
        "baseIri": base_iri, "eli": inst.eli, "schemaVersion": args.schema_version,
        "curationStatus": "cml:Proposed",
        "generatedBy": {"tool": TOOL_IRI, "model": None if args.no_llm else args.model},
        "generatedAt": datetime.now(timezone.utc).isoformat(),
        "source": {"file": source_ref, "originalName": args.input.name,
                   "sha256": source_hash, "pages": len(pages), "linked": bool(args.link)},
        "segmentation": {**config, "segments": len(segments),
                         "completed": len(progress.get("completed", {})),
                         "pending": len(failures)},
        "logicSuite": {
            "consideredForEverySegment": list(LOGIC_SUITE),
            "selectionPolicy": "subject-matter-selected; machine-proposed; omit when inapplicable",
            "minimumConfidence": MIN_LOGIC_CONFIDENCE,
            "applications": logic_counts,
            "cogAiChunks": cogai_count,
            "shaclShape": shacl_path.name,
            "alwaysAppliedInfrastructure": ["CML", "N3", "SHACL", "CogAI serialization"],
        },
        "counts": {"sections": len(sections), "subsections": len(subsections),
                   "provisions": len(units),
                   "concepts": len(concept_units(provisions)),
                   "structural": sum(1 for p in provisions
                                     if p.kind in ("part", "division", "schedule")),
                   "withText": sum(1 for p in concept_units(provisions)
                                   if provision_source_text(p).strip()),
                   "emptyText": sum(1 for p in concept_units(provisions)
                                    if not provision_source_text(p).strip()),
                   "classified": sum(1 for p in units if classification_for(p, cls_by_num))},
        "coverage": coverage_report(inst),
        "metadata": {"actNumber": inst.act_no, "year": inst.year,
                     "longTitle": inst.long_title, "date": inst.date},
        "validation": {"rdf": rdf_validation, "qualiaDb": qualia_validation},
        "files": generated_files,
        "note": "Machine-proposed layer. Regeneration is non-destructive; human cml:Attested overlays are never written by this tool. Every concept carries values:originalText when body text was extracted.",
    }
    write_json_atomic(out_dir / "manifest.json", manifest)

    cov = manifest["coverage"]
    print(f"· package written to {out_dir}/  "
          f"({manifest['counts']['classified']}/{len(units)} classified; "
          f"{cov['conceptsWithText']}/{cov['concepts']} concepts with originalText; "
          f"empty={cov['emptyConcepts']})")
    if failures:
        raise SystemExit(2)


if __name__ == "__main__":
    main()
