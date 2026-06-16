#!/usr/bin/env python3
"""Scan W3C namespace archives, dedupe serializations, diff against bundled catalogs."""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DEFAULT_ARCHIVES = Path(r"C:\Projects\ontologies-2023\w3c archives")

SEMANTIC_EXTS = {".ttl", ".rdf", ".n3", ".jsonld", ".owl", ".nt"}
FORMAT_RANK = {".ttl": 0, ".nt": 1, ".rdf": 2, ".owl": 3, ".n3": 4, ".jsonld": 5}

SKIP_DIRS = {
    "iana",
    "activitystreams-history",
    "radion_files",
    "test-bed",
    "plugfest",
    "examples",
    "example",
}

VERSION_RE = re.compile(r"(?:^|[-_])(?:20\d{6}|v?\d+(?:\.\d+)+|\d{8})(?:$|[-_.])", re.I)
DATE_SUFFIX_RE = re.compile(r"-20\d{6}$|-\d{8}$", re.I)

NS_PATTERNS = [
    re.compile(r"vann:preferredNamespaceUri\s+<?([^>\"'\s]+)>?", re.I),
    re.compile(r"xml:base=\"([^\"]+)\"", re.I),
    re.compile(r"@base\s*<([^>]+)>", re.I),
    re.compile(r"@prefix\s+\w+:\s*<(https?://[^>]+)>", re.I),
    re.compile(r"<(https?://www\.w3\.org/[^>]+)>\s+a\s+owl:Ontology", re.I),
]

VOCAB_DEFAULT_NS = {
    "acl": "http://www.w3.org/ns/auth/acl#",
    "adms": "http://www.w3.org/ns/adms#",
    "cert": "https://www.w3.org/ns/auth/cert#",
    "ldp": "http://www.w3.org/ns/ldp#",
    "locn": "http://www.w3.org/ns/locn#",
    "dqv": "http://www.w3.org/ns/dqv#",
    "duv": "http://www.w3.org/ns/duv#",
    "earl": "https://www.w3.org/ns/earl#",
    "oa": "http://www.w3.org/ns/oa#",
    "rsa": "https://www.w3.org/ns/auth/rsa#",
    "sparql": "http://www.w3.org/ns/sparql#",
    "shex": "http://www.w3.org/ns/shex#",
    "shapetrees": "http://www.w3.org/ns/shapetrees#",
    "webmention": "http://www.w3.org/ns/webmention#",
    "r2rml": "http://www.w3.org/ns/r2rml#",
    "mls": "http://www.w3.org/ns/mls#",
    "footprints": "http://www.w3.org/ns/footprints#",
    "regorg": "http://www.w3.org/ns/regorg#",
    "person": "http://www.w3.org/ns/person#",
    "spec": "http://www.w3.org/ns/spec#",
    "mblog": "http://www.w3.org/ns/mblog#",
    "mics": "http://www.w3.org/ns/mics#",
    "activitystreams-owl": "http://www.w3.org/ns/activitystreams#",
    "td": "https://www.w3.org/2019/wot/td#",
    "wot-security": "https://www.w3.org/2019/wot/security#",
    "overview": "http://www.w3.org/ns/sosa-prov/",
}


def load_existing_namespaces() -> tuple[set[str], set[str]]:
    known: set[str] = set()
    bundled_stems: set[str] = set()
    for rel in (
        "bundled/ontologies/w3c/catalog.json",
        "bundled/ontologies/purl/catalog.json",
        "bundled/ontologies/dublincore/catalog.json",
        "bundled/ontologies/geonames/catalog.json",
        "bundled/ontologies/fibo/catalog.json",
    ):
        path = REPO / rel
        if not path.is_file():
            continue
        data = json.loads(path.read_text(encoding="utf-8"))
        for entry in data.get("ontologies", []):
            known.add(normalize_ns_key(entry.get("namespace", "")))
            bundled_stems.add(Path(entry.get("file", "")).stem.lower())
    return known, bundled_stems


def normalize_ns(ns: str) -> str:
    ns = ns.strip().strip("<>").strip('"').strip("'")
    if not ns:
        return ""
    if not (ns.endswith("#") or ns.endswith("/")):
        ns += "#"
    return ns


def normalize_ns_key(ns: str) -> str:
    key = normalize_ns(ns).rstrip("#/").lower()
    key = key.replace("http://w3.org/", "http://www.w3.org/")
    key = key.replace("https://w3.org/", "https://www.w3.org/")
    return key


def extract_namespace(path: Path, head: str) -> str:
    stem = path.stem.lower()
    if stem in VOCAB_DEFAULT_NS:
        return normalize_ns(VOCAB_DEFAULT_NS[stem])
    for pat in NS_PATTERNS:
        for m in pat.finditer(head):
            candidate = normalize_ns(m.group(1))
            if "w3.org" in candidate:
                return candidate
    parent = path.parent.name.lower()
    if parent not in ("w3c-ns", "ontology", "context", "ns-main", "2"):
        return normalize_ns(f"https://www.w3.org/ns/{parent}/{stem}#")
    return ""


def valid_namespace(ns: str) -> bool:
    key = normalize_ns_key(ns)
    if not key.startswith("http"):
        return False
    if any(x in key for x in ("192.168.", "localhost", "example.com", "xmlns.com")):
        return False
    return "w3.org" in key


def file_score(path: Path) -> tuple:
    name = path.stem.lower()
    version_penalty = 2 if VERSION_RE.search(name) else 0
    if DATE_SUFFIX_RE.search(name):
        version_penalty += 3
    generic_bonus = 0
    if name in ("overview", "profile", "entailment", "sosa", "ssn", "odrl", "acl", "td", "locn", "ldp"):
        generic_bonus = -2
    if any(x in name for x in ("inverse", "refinement", "directmapping", "external", "2014", "dcat2")):
        generic_bonus += 2
    if name.endswith("-context") or "context" in name:
        generic_bonus += 4
    return (
        version_penalty,
        FORMAT_RANK.get(path.suffix.lower(), 9),
        generic_bonus,
        len(name),
        name,
    )


def slugify(*parts: str) -> str:
    raw = "-".join(p for p in parts if p)
    return re.sub(r"[^a-z0-9]+", "-", raw.lower()).strip("-")


def label_from_slug(slug: str) -> str:
    special = {
        "ldp": "LDP",
        "oa": "Open Annotation",
        "odrl": "ODRL",
        "qb4st": "QB4ST",
        "r2rml": "R2RML",
        "shex": "ShEx",
        "wot-td": "WoT TD",
        "wot-hctl": "WoT HCTL",
    }
    return special.get(slug, slug.replace("-", " ").title())


def prefix_from_ns(ns: str) -> str:
    core = ns.rstrip("#/").split("/")[-1]
    core = re.sub(r"[^a-zA-Z0-9]", "", core) or "voc"
    return core[:12].lower()


def default_search_from_file(path: Path, head: str) -> str:
    m = re.search(r"owl:Class\s+(?:\w+:)?(\w+)", head, re.I)
    if m:
        return m.group(1)
    return label_from_slug(path.stem)


def should_skip_path(path: Path, archives: Path) -> bool:
    rel = path.relative_to(archives)
    if any(part.lower() in SKIP_DIRS for part in rel.parts):
        return True
    if path.suffix.lower() == ".jsonld" and (
        path.name.startswith("sha256hex-") or "context" in path.name.lower()
    ):
        return True
    if "plugfest" in str(rel).lower():
        return True
    return False


def stem_family(stem: str) -> str:
    base = DATE_SUFFIX_RE.sub("", stem)
    base = re.sub(r"[-_]20\d{6}$", "", base)
    base = re.sub(r"[-_]?\d{8}$", "", base)
    return base.lower() or stem.lower()


def collect_semantic_files(root: Path, archives: Path) -> list[Path]:
    return [
        p
        for p in root.rglob("*")
        if p.is_file()
        and p.suffix.lower() in SEMANTIC_EXTS
        and not should_skip_path(p, archives)
        and p.stat().st_size >= 20
    ]


def vocab_units(archives: Path) -> list[tuple[str, list[Path]]]:
    units: dict[str, list[Path]] = {}
    w3c_ns = archives / "ns-main" / "w3c-ns"
    if w3c_ns.is_dir():
        root_files = [
            p for p in w3c_ns.iterdir() if p.is_file() and p.suffix.lower() in SEMANTIC_EXTS
        ]
        root_groups: dict[str, list[Path]] = {}
        for path in root_files:
            root_groups.setdefault(stem_family(path.stem), []).append(path)
        for family, files in root_groups.items():
            units[family] = files

        for child in sorted(w3c_ns.iterdir()):
            if not child.is_dir() or child.name in SKIP_DIRS:
                continue
            for path in collect_semantic_files(child, archives):
                rel = path.relative_to(child)
                if len(rel.parts) > 2:
                    continue
                key = slugify(child.name, stem_family(path.stem))
                units.setdefault(key, []).append(path)

    wot_ont = archives / "wot-thing-description" / "ontology"
    if wot_ont.is_dir():
        for path in sorted(wot_ont.iterdir()):
            if path.is_file() and path.suffix.lower() in SEMANTIC_EXTS:
                units[slugify("wot", path.stem)] = [path]

    return list(units.items())


def pick_canonical(files: list[Path]) -> Path | None:
    if not files:
        return None
    files = sorted(files, key=file_score)
    return files[0]


def build_catalog(archives: Path, known_ns: set[str], bundled_stems: set[str]) -> dict:
    entries = []
    used_ids: set[str] = set()
    seen_ns: set[str] = set()

    for slug, files in vocab_units(archives):
        path = pick_canonical(files)
        if path is None:
            continue
        if stem_family(path.stem) in bundled_stems:
            continue
        head = path.read_text(encoding="utf-8", errors="replace")[:160_000]
        ns = extract_namespace(path, head)
        if not ns or not valid_namespace(ns):
            continue
        ns_key = normalize_ns_key(ns)
        if ns_key in known_ns or ns_key in seen_ns:
            continue
        seen_ns.add(ns_key)

        entry_id = f"w3c-arch-{slug}"
        n = 2
        while entry_id in used_ids:
            entry_id = f"w3c-arch-{slug}-{n}"
            n += 1
        used_ids.add(entry_id)

        ext = path.suffix.lower()
        entries.append(
            {
                "file": f"{slug}{ext}",
                "source": str(path.relative_to(archives)).replace("\\", "/"),
                "id": entry_id,
                "label": label_from_slug(slug),
                "icon": "📦",
                "prefix": prefix_from_ns(ns),
                "namespace": ns,
                "defaultSearch": default_search_from_file(path, head),
                "quickExamples": [],
            }
        )

    entries.sort(key=lambda e: e["label"].lower())
    return {
        "version": 1,
        "source": "bundled/ontologies/w3c-archives",
        "license": "W3C",
        "archives_root": str(archives),
        "skipped": sorted(SKIP_DIRS) + ["iana/media-types (~2k duplicate serializations)"],
        "ontologies": entries,
    }


def sync_sources(catalog: dict, archives: Path, dest_dir: Path) -> int:
    dest_dir.mkdir(parents=True, exist_ok=True)
    for old in dest_dir.iterdir():
        if old.name == "catalog.json":
            continue
        if old.is_file():
            old.unlink()
    copied = 0
    for entry in catalog.get("ontologies", []):
        src = archives / entry["source"]
        dst = dest_dir / entry["file"]
        if not src.is_file():
            print(f"  missing source: {entry['source']}", file=sys.stderr)
            continue
        dst.write_bytes(src.read_bytes())
        copied += 1
    return copied


def main() -> int:
    archives = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_ARCHIVES
    if not archives.is_dir():
        print(f"Archives not found: {archives}", file=sys.stderr)
        return 1

    known, bundled_stems = load_existing_namespaces()
    catalog = build_catalog(archives, known, bundled_stems)
    dest = REPO / "bundled/ontologies/w3c-archives"
    copied = sync_sources(catalog, archives, dest)
    out = dest / "catalog.json"
    out.write_text(json.dumps(catalog, indent=2) + "\n", encoding="utf-8")

    print(f"archives: {archives}")
    print(f"new vocabularies: {len(catalog['ontologies'])}")
    print(f"copied sources: {copied} -> {dest}")
    print(f"catalog: {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())