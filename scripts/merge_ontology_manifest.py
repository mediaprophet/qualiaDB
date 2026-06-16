#!/usr/bin/env python3
"""Merge bundled ontology catalog entries into docs/playground/vfs-manifest.json."""
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
MANIFEST = REPO / "docs/playground/vfs-manifest.json"

GROUP_CONFIG = {
    "w3c": {
        "src": REPO / "bundled/ontologies/w3c",
        "out": REPO / "docs/data/w3c",
        "url_prefix": "data/w3c",
        "profile": "w3c",
        "license": "W3C",
        "homepage": "https://www.w3.org/standards/semanticweb/ontology",
        "description": lambda e: f"W3C {e['label']} vocabulary · {e['file']}",
    },
    "purl": {
        "src": REPO / "bundled/ontologies/purl",
        "out": REPO / "docs/data/purl",
        "url_prefix": "data/purl",
        "profile": "w3c",
        "license": "PURL.org",
        "homepage": "https://purl.org/",
        "description": lambda e: f"PURL {e['label']} vocabulary · {e['file']}",
    },
    "geonames": {
        "src": REPO / "bundled/ontologies/geonames",
        "out": REPO / "docs/data/geonames",
        "url_prefix": "data/geonames",
        "profile": "w3c",
        "license": "CC-BY 3.0",
        "homepage": "https://www.geonames.org/ontology/",
        "description": lambda e: f"GeoNames geography ontology · {e['file']} · ~1.4k triples",
    },
    "dublincore": {
        "src": REPO / "bundled/ontologies/dublincore",
        "out": REPO / "docs/data/dublincore",
        "url_prefix": "data/dublincore",
        "profile": "w3c",
        "license": "CC-BY 4.0",
        "homepage": "https://www.dublincore.org/",
        "description": lambda e: f"DCMI metadata terms · {e['file']} · combined Dublin Core export",
    },
    "fibo": {
        "src": REPO / "bundled/ontologies/fibo",
        "out": REPO / "docs/data/fibo",
        "url_prefix": "data/fibo",
        "profile": "w3c",
        "license": "MIT",
        "homepage": "https://spec.edmcouncil.org/fibo/",
        "description": lambda e: f"EDMC FIBO {e['label']} · {e.get('domain', '')} domain",
    },
    "w3c-archives": {
        "src": REPO / "bundled/ontologies/w3c-archives",
        "out": REPO / "docs/data/w3c-archives",
        "url_prefix": "data/w3c-archives",
        "profile": "w3c",
        "license": "W3C",
        "homepage": "https://www.w3.org/standards/semanticweb/ontology",
        "description": lambda e: f"W3C archives {e['label']} · deduped from ns.w3.org mirror",
    },
}


def build_entries(group: str) -> list:
    cfg = GROUP_CONFIG[group]
    catalog = json.loads((cfg["src"] / "catalog.json").read_text(encoding="utf-8"))
    built = []
    for entry in catalog.get("ontologies", []):
        if entry.get("domain"):
            base = entry["domain"].lower()
        else:
            base = Path(entry["file"]).stem
        if not (cfg["out"] / f"{base}.q42").is_file():
            continue
        term = entry.get("defaultSearch", "Resource")
        ns = entry["namespace"]
        ds = {
            "id": entry["id"],
            "profile": cfg["profile"],
            "label": entry["label"],
            "icon": entry.get("icon", "📘"),
            "description": cfg["description"](entry),
            "url": f"{cfg['url_prefix']}/{base}.q42",
            "lexUrl": f"{cfg['url_prefix']}/{base}.q42.lex",
            "bidxUrl": f"{cfg['url_prefix']}/{base}.q42.bidx",
            "compressed": False,
            "source": "bundled",
            "license": entry.get("license", catalog.get("license", cfg["license"])),
            "homepage": entry.get("homepage", cfg["homepage"]),
            "namespace": ns,
            "prefix": entry["prefix"],
            "defaultSearch": term,
            "quickExamples": entry.get("quickExamples", []),
            "sampleQueries": [
                {"label": f"Inspect {term}", "pattern": f"<{ns}{term}> ?p ?o"},
                {
                    "label": "Subclass chain",
                    "pattern": (
                        f"<{ns}{term}> "
                        "<http://www.w3.org/2000/01/rdf-schema#subClassOf> ?o"
                    ),
                },
            ],
        }
        if not entry.get("primary"):
            ds["group"] = group
        built.append(ds)
    return built


def merge_group(group: str) -> int:
    built = build_entries(group)
    built_ids = {d["id"] for d in built}
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    stripped = [
        d for d in manifest.get("datasets", [])
        if d.get("group") != group and d.get("id") not in built_ids
    ]
    manifest["datasets"] = stripped + built
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"manifest: {len(built)} {group} datasets")
    return len(built)


if __name__ == "__main__":
    groups = sys.argv[1:] or ["w3c"]
    for g in groups:
        if g not in GROUP_CONFIG:
            raise SystemExit(f"Unknown group: {g} (expected: {', '.join(GROUP_CONFIG)})")
        merge_group(g)