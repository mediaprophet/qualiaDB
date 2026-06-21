#!/usr/bin/env python3
"""
Scrape a person's posts from the W3C public mailing-list archives and emit RDF (Turtle),
so the record can be ingested into a self-owned .q42 vault (provenance you hold, not a platform).

This is *defensive provenance* tooling: it archives an author's own public contributions into a
structured, queryable, cryptographically-anchorable form. It only touches W3C's PUBLIC archives.

Pipeline:
    python scripts/scrape_w3c_mail_to_rdf.py --email timothy.holborn@gmail.com --out w3c_holborn.ttl
    ./target/release/qualia-cli ingest semantic w3c_holborn.ttl       # -> w3c_holborn.q42
(or run scripts/build_w3c_mail_q42.sh which does both)

How it works: the W3C mail *search* lists each hit as a structured block carrying Subject, a stable
`https://www.w3.org/mid/<message-id>` permalink, List, Date and Author. We parse those blocks across
all result pages (fast, robust) and — by default — keep only messages whose Author is the target
address (your authorship). Full message bodies are fetched only with --bodies (slower, resumable).

RDF model (standards-based — SIOC posts, FOAF people, DCTerms metadata, PROV attribution):

    <https://www.w3.org/mid/ID;list=L> a sioc:Post ;
        dcterms:title    "subject" ;
        dcterms:created  "2014-05-21T23:45:52+10:00"^^xsd:dateTime ;
        sioc:has_creator <person/<hash>> ;
        sioc:has_container <list/public-webpayments> ;
        schema:abstract  "search snippet" ;          # always
        sioc:content     "<full body>" ;             # only with --bodies
        sioc:reply_of    <parent-mid> ;              # only with --bodies, when resolvable
        schema:identifier "<message-id>" ;
        prov:wasAttributedTo <person/<hash>> .

Robustness: per-URL disk cache (resumable, never re-hits a fetched page), polite delay + retry/backoff,
identifying User-Agent, --limit for testing.
"""
import argparse
import email.utils
import hashlib
import html
import os
import re
import sys
import time
import urllib.parse
import urllib.request
from datetime import datetime, timezone

# Windows consoles default to cp1252 and crash on Unicode in subjects/bodies/arrows.
try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")
except Exception:  # noqa: BLE001 — older/odd stdouts; best-effort
    pass

try:
    from rdflib import Graph, Namespace, URIRef, Literal, RDF
    from rdflib.namespace import XSD, FOAF, DCTERMS
except ImportError:
    sys.exit("rdflib required: pip install rdflib")

SIOC = Namespace("http://rdfs.org/sioc/ns#")
PROV = Namespace("http://www.w3.org/ns/prov#")
SCHEMA = Namespace("http://schema.org/")

SEARCH_URL = "https://www.w3.org/Search/Mail/Public/search"
UA = "QualiaDB-provenance-archiver/1.0 (self-archival of own public W3C posts; +https://github.com/WebizenAI)"

# One search-result hit: the txt-mars <h2> permalink + the following <dl> (List/Date/Author).
RE_RESULT = re.compile(
    r'<h2 class="txt-mars">\s*<a href="([^"]+)"[^>]*>(.*?)</a>\s*</h2>(.*?)</dl>', re.S | re.I)
RE_SNIPPET = re.compile(r'</dl>\s*<p>(.*?)</p>', re.S | re.I)
RE_DD = {
    "list": re.compile(r'<dt>\s*List\s*</dt>\s*<dd>\s*(?:<a[^>]*>)?\s*([^<]+)', re.I),
    "date": re.compile(r'<dt>\s*Date\s*</dt>\s*<dd>\s*([^<]+)', re.I),
    "author": re.compile(r'<dt>\s*Author\s*</dt>\s*<dd>\s*([^<]+)', re.I),
}
RE_TOTAL = re.compile(r'results?\s+[\d,]+-[\d,]+\s+of\s+([\d,]+)', re.I)
RE_TAG = re.compile(r"<[^>]+>")
# Full-message body enrichment (hypermail comments — confirmed present in W3C archives):
RE_ISOSENT = re.compile(r'<!--\s*isosent="(\d{14})"\s*-->', re.I)
RE_INREPLY = re.compile(r'<!--\s*inreplyto="([^"]*)"\s*-->', re.I)
RE_BODY = re.compile(r'<!--\s*body="start"\s*-->(.*?)<!--\s*body="end"\s*-->', re.I | re.S)
RE_PRE = re.compile(r"<pre[^>]*>(.*?)</pre>", re.I | re.S)


def fetch(url, cache_dir, delay, retries=4, timeout=45):
    """GET url with disk cache (resumable) + polite delay + backoff."""
    key = hashlib.sha1(url.encode()).hexdigest() + ".html"
    path = os.path.join(cache_dir, key)
    if os.path.exists(path):
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            return f.read()
    last = None
    for attempt in range(retries):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept-Encoding": "identity"})
            with urllib.request.urlopen(req, timeout=timeout) as r:
                data = r.read().decode("utf-8", errors="replace")
            os.makedirs(cache_dir, exist_ok=True)
            with open(path, "w", encoding="utf-8") as f:
                f.write(data)
            time.sleep(delay)  # be a good citizen — only after a real network hit
            return data
        except Exception as e:  # noqa: BLE001 — transient network/HTTP; retry with backoff
            last = e
            time.sleep(delay * (2 ** attempt))
    print(f"  ! fetch failed ({last}): {url}", file=sys.stderr)
    return None


def parse_author(dd):
    """'Timothy Holborn <timothy.holborn@gmail.com>' -> (name, email)."""
    s = html.unescape(dd).strip()
    m = re.search(r"<([^>]+@[^>]+)>", s)
    em = m.group(1).strip().lower() if m else None
    name = re.sub(r"\s*<[^>]+>\s*", "", s).strip() or None
    return name, em


def rfc822_to_xsd(d):
    try:
        dt = email.utils.parsedate_to_datetime(d.strip())
        return dt.isoformat() if dt else None
    except Exception:  # noqa: BLE001
        return None


def parse_results(page):
    """Yield one dict per search-result block on a results page."""
    out = []
    for m in RE_RESULT.finditer(page):
        url, subj_html, dl = m.group(1), m.group(2), m.group(3)
        author = RE_DD["author"].search(dl)
        name, em = parse_author(author.group(1)) if author else (None, None)
        lst = RE_DD["list"].search(dl)
        date = RE_DD["date"].search(dl)
        msgid = None
        um = re.search(r"/mid/([^;?\"]+)", url)
        if um:
            msgid = urllib.parse.unquote(um.group(1))
        snip = RE_SNIPPET.search(page[m.start():m.start() + len(m.group(0)) + 1500])
        out.append({
            "url": url,
            "subject": html.unescape(RE_TAG.sub("", subj_html)).strip() or None,
            "list": html.unescape(lst.group(1)).strip() if lst else None,
            "date": rfc822_to_xsd(date.group(1)) if date else None,
            "name": name, "email": em, "msgid": msgid,
            "snippet": html.unescape(RE_TAG.sub("", snip.group(1))).strip() if snip else None,
        })
    return out


def enrich_body(rec, cache_dir, delay):
    """Fetch the message page; return (body_text, inreplyto_msgid)."""
    page = fetch(rec["url"], cache_dir, delay)
    if not page:
        return None, None
    bm = RE_BODY.search(page)
    chunk = bm.group(1) if bm else (RE_PRE.search(page).group(1) if RE_PRE.search(page) else "")
    body = html.unescape(RE_TAG.sub("", chunk)).strip() or None
    irt = RE_INREPLY.search(page)
    irt = html.unescape(irt.group(1)).strip() if irt else None
    return body, irt


def person_uri(base, email, name):
    h = hashlib.sha1((email or name or "anon").lower().encode()).hexdigest()[:16]
    return URIRef(f"{base}person/{h}")


def main():
    ap = argparse.ArgumentParser(description="Scrape W3C public mailing-list posts -> RDF/Turtle.")
    ap.add_argument("--email", default="timothy.holborn@gmail.com", help="author email to archive")
    ap.add_argument("--out", default=None, help="output .ttl (default w3c_<localpart>.ttl)")
    ap.add_argument("--cache", default=".w3c_mail_cache", help="HTML cache dir (resumable)")
    ap.add_argument("--delay", type=float, default=1.0, help="seconds between network requests")
    ap.add_argument("--limit", type=int, default=0, help="cap results processed (0 = all) — for testing")
    ap.add_argument("--max-pages", type=int, default=60, help="max search result pages to walk")
    ap.add_argument("--include-mentions", action="store_true",
                    help="include messages that merely CC/quote the address (default: authored only)")
    ap.add_argument("--bodies", action="store_true",
                    help="also fetch each message page for full body + reply linkage (slower, resumable)")
    ap.add_argument("--base", default="https://webizen.org/archive/w3c-mail/", help="RDF resource base URI")
    ap.add_argument("--format", choices=["turtle", "nt", "both"], default="both",
                    help="output RDF format. 'both' writes .ttl (human-readable) + .nt "
                         "(N-Triples — lossless for `qualia-cli ingest`, whose Turtle parser is partial)")
    args = ap.parse_args()

    out = args.out or f"w3c_{args.email.split('@')[0].replace('.', '_')}.ttl"
    target = args.email.lower()

    print(f"=== W3C mail -> RDF for {args.email} ===")
    print("Walking W3C public mail search result pages ...")
    results, seen, total = [], set(), None
    for page_no in range(1, args.max_pages + 1):
        q = urllib.parse.urlencode({
            "keywords": args.email, "indexes": "Public",
            "resultsperpage": "100", "sortby": "date-asc", "page": str(page_no),
        })
        page = fetch(f"{SEARCH_URL}?{q}", args.cache, args.delay)
        if not page:
            break
        if total is None:
            tm = RE_TOTAL.search(page)
            total = tm.group(1) if tm else "?"
        block = parse_results(page)
        new = [r for r in block if r["url"] not in seen]
        for r in new:
            seen.add(r["url"])
        results.extend(new)
        kept = sum(1 for r in block if r["email"] == target)
        print(f"  page {page_no}: {len(block)} hits ({kept} by {args.email}), total kept so far {len(results)}")
        if not block or not new:
            break

    print(f"search reports {total} total hits mentioning the address.")
    if not args.include_mentions:
        results = [r for r in results if r["email"] == target]
        print(f"filtered to {len(results)} messages AUTHORED by {args.email}.")
    if args.limit:
        results = results[: args.limit]

    g = Graph()
    for p, ns in [("sioc", SIOC), ("foaf", FOAF), ("dcterms", DCTERMS),
                  ("prov", PROV), ("schema", SCHEMA), ("xsd", XSD)]:
        g.bind(p, ns)
    base = args.base
    people, lists, by_year, msgid_to_uri = {}, set(), {}, {}

    if args.bodies:
        print(f"Fetching {len(results)} message bodies (resumable cache) ...")
    for i, r in enumerate(results, 1):
        if r["msgid"]:
            msgid_to_uri[r["msgid"]] = URIRef(r["url"])
    for i, r in enumerate(results, 1):
        s = URIRef(r["url"])
        g.add((s, RDF.type, SIOC.Post))
        if r["subject"]:
            g.add((s, DCTERMS.title, Literal(r["subject"])))
        if r["date"]:
            g.add((s, DCTERMS.created, Literal(r["date"], datatype=XSD.dateTime)))
            by_year[r["date"][:4]] = by_year.get(r["date"][:4], 0) + 1
        if r["snippet"]:
            g.add((s, SCHEMA.abstract, Literal(r["snippet"])))
        if r["msgid"]:
            g.add((s, SCHEMA.identifier, Literal(r["msgid"])))
        if r["list"]:
            lst = URIRef(f"{base}list/{r['list']}")
            lists.add(r["list"])
            g.add((s, SIOC.has_container, lst))
            g.add((lst, RDF.type, SIOC.Forum))
            g.add((lst, DCTERMS.title, Literal(r["list"])))
        p = person_uri(base, r["email"], r["name"])
        if (r["email"] or r["name"]) and p not in people:
            people[p] = True
            g.add((p, RDF.type, FOAF.Person))
            if r["name"]:
                g.add((p, FOAF.name, Literal(r["name"])))
            if r["email"]:
                g.add((p, FOAF.mbox, URIRef("mailto:" + r["email"])))
        g.add((s, SIOC.has_creator, p))
        g.add((s, PROV.wasAttributedTo, p))
        if args.bodies:
            body, irt = enrich_body(r, args.cache, args.delay)
            if body:
                g.add((s, SIOC.content, Literal(body)))
            if irt and irt in msgid_to_uri:
                g.add((s, SIOC.reply_of, msgid_to_uri[irt]))
            if i % 25 == 0:
                print(f"    ... {i}/{len(results)} bodies")

    g.add((URIRef(base), DCTERMS.created,
           Literal(datetime.now(timezone.utc).replace(microsecond=0).isoformat())))

    stem = os.path.splitext(out)[0]
    ttl_path, nt_path = stem + ".ttl", stem + ".nt"
    written = []
    if args.format in ("turtle", "both"):
        g.serialize(destination=ttl_path, format="turtle"); written.append(ttl_path)
    if args.format in ("nt", "both"):
        g.serialize(destination=nt_path, format="nt"); written.append(nt_path)
    ingest_src = nt_path if args.format in ("nt", "both") else ttl_path

    print(f"\n=== done ===")
    print(f"messages archived : {len(results)}")
    print(f"distinct people   : {len(people)}")
    print(f"mailing lists     : {len(lists)}  ({', '.join(sorted(lists))})")
    if by_year:
        print("by year           : " + ", ".join(f"{y}:{n}" for y, n in sorted(by_year.items())))
    print(f"triples           : {len(g)}")
    print(f"RDF written       : {', '.join(written)}")
    print(f"\nNext: ./target/release/qualia-cli ingest semantic {ingest_src}   # -> {stem}.q42")
    if args.format == "turtle":
        print("  (note: the CLI's Turtle parser is partial — use --format nt/both for a complete .q42)")


if __name__ == "__main__":
    main()
