#!/usr/bin/env python3
"""
Consolidate the separate provenance graphs into ONE master vault, fix third-party attribution,
and fold in the current capstone (Peace Infrastructure Alliance). Produces master.ttl/.nt for
`qualia-cli ingest semantic` -> master.q42.

Merges: provenance/w3c_timothy_holborn.nt + docs_holborn.nt + article_egalitarian_meritocracy.nt
Adds:   the Peace Infrastructure Alliance markdown (text cleaned of embedded base64 images)
Fixes:  testimonials/correspondence wrongly attributed to Holborn -> attributed to their real
        authors, marked as being ABOUT Holborn (schema:about) — that's where their value lies.
"""
import hashlib, os, re, sys
from rdflib import Graph, Namespace, URIRef, Literal, RDF
from rdflib.namespace import XSD, FOAF, DCTERMS

SCHEMA = Namespace("http://schema.org/"); PROV = Namespace("http://www.w3.org/ns/prov#")
base = "https://webizen.org/archive/docs/"
HOLBORN = URIRef(base + "person/timothy-holborn")

g = Graph()
for p, ns in [("schema", SCHEMA), ("prov", PROV), ("dcterms", DCTERMS), ("foaf", FOAF), ("xsd", XSD)]:
    g.bind(p, ns)

# 1. merge existing graphs
for nt in ["provenance/w3c_timothy_holborn.nt", "provenance/docs_holborn.nt",
           "provenance/article_egalitarian_meritocracy.nt"]:
    if os.path.exists(nt):
        before = len(g); g.parse(nt, format="nt")
        print(f"  merged {nt}: +{len(g)-before} triples")

# 2. fold in the Peace Infrastructure Alliance capstone (text cleaned of base64 image blobs)
md_path = "C:/Users/Admin/Downloads/The peace infrastructure alliance.md"
if os.path.exists(md_path):
    raw = open(md_path, "rb").read()
    txt = raw.decode("utf-8", "replace")
    txt = re.sub(r'!\[[^\]]*\]\([^)]*\)', ' ', txt)             # markdown images
    txt = re.sub(r'data:[^)\s]+', ' ', txt)                      # data: URIs
    txt = re.sub(r'\[image\d+\]:\s*<[^>]*>', ' ', txt)           # image refs
    txt = re.sub(r'[A-Za-z0-9+/]{200,}={0,2}', ' ', txt)         # stray base64 runs
    txt = re.sub(r'[ \t]+', ' ', re.sub(r'\n{3,}', '\n\n', txt)).strip()
    s = URIRef(base + "doc/peace-infrastructure-alliance")
    g.add((s, RDF.type, SCHEMA.CreativeWork))
    g.add((s, DCTERMS.title, Literal("The Peace Infrastructure Alliance")))
    g.add((s, DCTERMS.created, Literal("2022-06-15", datatype=XSD.date)))
    g.add((s, SCHEMA.author, HOLBORN)); g.add((s, PROV.wasAttributedTo, HOLBORN))
    g.add((s, SCHEMA.sha256, Literal(hashlib.sha256(raw).hexdigest())))
    g.add((s, SCHEMA.wordCount, Literal(len(txt.split()), datatype=XSD.integer)))
    g.add((s, SCHEMA.text, Literal(txt)))
    g.add((s, DCTERMS.description, Literal(
        "Current capstone (Google Doc started 2022-06-15, ongoing): housing-system-as-IP-pool / "
        "'home as a platform', Trust Factory, Cyber Peace Fair, knowledge equity & human agency, "
        "Flux Accounting, addressed to AU Ministers & investors. Synthesises the 2000-> arc into a "
        "nation-building proposal; states the personal cost ('digital slavery', poverty) plainly.")))
    print(f"  added Peace Infrastructure Alliance: {len(txt.split()):,} words")

# 3. fix third-party attribution (testimonials / correspondence are NOT authored by Holborn)
THIRD_PARTY = {
    "Looms": ("Peter Olaf Looms", "testimonial about Timothy Holborn (Danmarks Radio / DTU / Univ. Hong Kong)"),
    "BillyDay": ("Billy Day", "reference about Timothy Holborn (finance expert, Hybrid TV standard)"),
    "manu": ("Manu Sporny", "correspondence with/by Manu Sporny (DID/credentials) involving Timothy Holborn"),
    "Manu": ("Manu Sporny", "correspondence with/by Manu Sporny (DID/credentials) involving Timothy Holborn"),
}
fixed = 0
for s, _, name in list(g.triples((None, SCHEMA.name, None))):
    fn = str(name)
    for key, (author, note) in THIRD_PARTY.items():
        if key in fn:
            # remove Holborn authorship
            g.remove((s, SCHEMA.author, HOLBORN)); g.remove((s, PROV.wasAttributedTo, HOLBORN))
            person = URIRef(base + "person/" + re.sub(r'\W+', '-', author.lower()))
            g.add((person, RDF.type, FOAF.Person)); g.add((person, FOAF.name, Literal(author)))
            g.add((s, SCHEMA.author, person)); g.add((s, PROV.wasAttributedTo, person))
            g.add((s, SCHEMA.about, HOLBORN))           # it is ABOUT Holborn
            g.add((s, DCTERMS.description, Literal("Third-party " + note)))
            fixed += 1
            break
print(f"  re-attributed {fixed} third-party item(s) to their real authors")

g.serialize("provenance/master.ttl", format="turtle")
g.serialize("provenance/master.nt", format="nt")
people = len(set(g.subjects(RDF.type, FOAF.Person)))
works = len(set(g.subjects(RDF.type, SCHEMA.CreativeWork))) + \
        len(set(g.subjects(RDF.type, SCHEMA.Article))) + \
        len(set(g.subjects(RDF.type, SCHEMA.ImageObject)))
posts = len(set(g.subjects(RDF.type, Namespace("http://rdfs.org/sioc/ns#").Post)))
print(f"\n=== master vault ===")
print(f"people: {people} | documents/works: {works} | mailing-list posts: {posts} | triples: {len(g)}")
print("written: provenance/master.ttl , provenance/master.nt")
