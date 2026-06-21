#!/usr/bin/env python3
"""
Turn a folder of an author's documents (.pdf/.docx) into RDF (Turtle + N-Triples), so the corpus
becomes an enumerated, attestable, self-owned .q42 provenance vault — authorship the author holds.

    python scripts/scrape_docs_to_rdf.py --dir C:/Projects/mydocs --out provenance/docs_holborn.ttl
    ./target/release/qualia-cli ingest semantic provenance/docs_holborn.nt   # -> .q42

Per document it records: title, the best available date (clearly labelled by SOURCE — embedded
metadata > the author's in-doc reference date e.g. *_20151122_* > a date encoded in the filename),
author attribution (PROV), word count, a SHA-256 content hash (tamper-evidence), and the full text.

Honesty about dating is built in: each date carries a `schema:description` saying where it came from,
so a "filename-implied" date is never silently presented as a verified authorship date.
"""
import argparse, glob, hashlib, html, os, re, subprocess, sys
from datetime import datetime, timezone

try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except Exception:
    pass
try:
    from rdflib import Graph, Namespace, URIRef, Literal, RDF
    from rdflib.namespace import XSD, FOAF, DCTERMS
except ImportError:
    sys.exit("rdflib required: pip install rdflib")

SCHEMA = Namespace("http://schema.org/")
PROV = Namespace("http://www.w3.org/ns/prov#")

RE_FNAME_DATE = re.compile(r'(?<!\d)(20\d{2})[-_ ]?(0[1-9]|1[0-2])?[-_ ]?(0[1-9]|[12]\d|3[01])?')
RE_INDOC_DATE = re.compile(r'(20\d{2})(0[1-9]|1[0-2])([0-3]\d)')  # e.g. TimothyHolborn_20151122_1.0


def pdf_text(path):
    try:
        return subprocess.run(["pdftotext", "-layout", path, "-"],
                              capture_output=True, text=True, timeout=120).stdout
    except Exception as e:  # noqa: BLE001
        print(f"  ! pdftotext failed: {os.path.basename(path)} ({e})", file=sys.stderr)
        return ""


def pdf_meta_date(path):
    try:
        out = subprocess.run(["pdfinfo", path], capture_output=True, text=True, timeout=30).stdout
        m = re.search(r'^CreationDate:\s*(.+)$', out, re.M)
        if m:
            d = datetime.strptime(m.group(1).strip()[:24], "%a %b %d %H:%M:%S %Y")
            return d.strftime("%Y-%m-%d")
    except Exception:  # noqa: BLE001
        pass
    return None


def docx_text_and_date(path):
    import zipfile
    text, created = "", None
    try:
        with zipfile.ZipFile(path) as z:
            core = z.read("docProps/core.xml").decode("utf-8", "replace")
            m = re.search(r'<dcterms:created[^>]*>([^<]+)', core)
            if m:
                created = m.group(1)[:10]
            doc = z.read("word/document.xml").decode("utf-8", "replace")
            doc = re.sub(r'<w:p[ >]', '\n', doc)
            text = html.unescape(re.sub(r'<[^>]+>', '', doc))
    except Exception as e:  # noqa: BLE001
        print(f"  ! docx parse failed: {os.path.basename(path)} ({e})", file=sys.stderr)
    return text, created


def best_date(fname, text, embedded):
    """Return (xsd_date, source_label)."""
    if embedded:
        return embedded, "embedded document metadata"
    m = RE_INDOC_DATE.search(text[:4000])  # author's own doc-reference stamp near the top
    if m:
        return f"{m.group(1)}-{m.group(2)}-{m.group(3)}", "author's in-document reference date"
    m = RE_FNAME_DATE.search(fname)
    if m and m.group(1):
        y, mo, d = m.group(1), m.group(2) or "01", m.group(3) or "01"
        return f"{y}-{mo}-{d}", "date encoded in the filename (author's naming convention) — not independently verified"
    return None, None


def main():
    ap = argparse.ArgumentParser(description="Documents -> RDF provenance vault.")
    ap.add_argument("--dir", default="C:/Projects/mydocs", help="folder of documents")
    ap.add_argument("--extra", nargs="*", default=[], help="extra individual files to include")
    ap.add_argument("--out", default="provenance/docs_holborn.ttl")
    ap.add_argument("--author", default="Timothy Charles Holborn")
    ap.add_argument("--email", default="timothy.holborn@gmail.com")
    ap.add_argument("--base", default="https://webizen.org/archive/docs/")
    args = ap.parse_args()

    files = sorted(set(glob.glob(os.path.join(args.dir, "*.pdf")) +
                       glob.glob(os.path.join(args.dir, "*.docx")) + args.extra))
    print(f"=== {len(files)} documents -> RDF ===")

    g = Graph()
    for p, ns in [("schema", SCHEMA), ("prov", PROV), ("dcterms", DCTERMS), ("foaf", FOAF), ("xsd", XSD)]:
        g.bind(p, ns)
    person = URIRef(f"{args.base}person/timothy-holborn")
    g.add((person, RDF.type, FOAF.Person))
    g.add((person, FOAF.name, Literal(args.author)))
    g.add((person, FOAF.mbox, URIRef("mailto:" + args.email)))

    years, undated, total_words = {}, 0, 0
    for path in files:
        fname = os.path.basename(path)
        data = open(path, "rb").read()
        sha = hashlib.sha256(data).hexdigest()
        if path.lower().endswith(".docx"):
            text, embedded = docx_text_and_date(path)
        else:
            text, embedded = pdf_text(path), pdf_meta_date(path)
        text = re.sub(r'[ \t]+', ' ', re.sub(r'\n{3,}', '\n\n', text)).strip()
        words = len(text.split())
        total_words += words
        date, src = best_date(fname, text, embedded)

        s = URIRef(args.base + "doc/" + hashlib.sha1(fname.encode()).hexdigest()[:16])
        g.add((s, RDF.type, SCHEMA.CreativeWork))
        g.add((s, DCTERMS.title, Literal(os.path.splitext(fname)[0])))
        g.add((s, SCHEMA.name, Literal(fname)))
        g.add((s, SCHEMA.author, person))
        g.add((s, PROV.wasAttributedTo, person))
        g.add((s, SCHEMA.sha256, Literal(sha)))
        g.add((s, SCHEMA.wordCount, Literal(words, datatype=XSD.integer)))
        if text:
            g.add((s, SCHEMA.text, Literal(text)))
        if date:
            g.add((s, DCTERMS.created, Literal(date, datatype=XSD.date)))
            g.add((s, SCHEMA.description, Literal(f"created date source: {src}")))
            years[date[:4]] = years.get(date[:4], 0) + 1
        else:
            undated += 1
        print(f"  • {fname[:60]:60}  {date or 'undated':10}  {words:>6} words")

    g.add((URIRef(args.base), DCTERMS.created,
           Literal(datetime.now(timezone.utc).replace(microsecond=0).isoformat())))
    stem = os.path.splitext(args.out)[0]
    os.makedirs(os.path.dirname(stem) or ".", exist_ok=True)
    g.serialize(destination=stem + ".ttl", format="turtle")
    g.serialize(destination=stem + ".nt", format="nt")

    print(f"\n=== done ===")
    print(f"documents : {len(files)}  (undated: {undated})")
    print(f"by year   : " + ", ".join(f"{y}:{n}" for y, n in sorted(years.items())))
    print(f"words     : {total_words:,}")
    print(f"triples   : {len(g)}")
    print(f"written   : {stem}.ttl , {stem}.nt")
    print(f"\nNext: ./target/release/qualia-cli ingest semantic {stem}.nt   # -> {stem}.q42")


if __name__ == "__main__":
    main()
