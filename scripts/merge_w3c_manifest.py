#!/usr/bin/env python3
"""Merge W3C catalog entries into docs/playground/vfs-manifest.json."""
import json
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
SRC = REPO / "bundled/ontologies/w3c"
OUT = REPO / "docs/data/w3c"
MANIFEST = REPO / "docs/playground/vfs-manifest.json"

catalog = json.loads((SRC / "catalog.json").read_text(encoding="utf-8"))
built = []

for entry in catalog.get("ontologies", []):
    base = Path(entry["file"]).stem
    if not (OUT / f"{base}.q42").is_file():
        continue
    term = entry.get("defaultSearch", "Resource")
    ns = entry["namespace"]
    built.append({
        "id": entry["id"],
        "group": "w3c",
        "profile": "w3c",
        "label": entry["label"],
        "icon": entry.get("icon", "📘"),
        "description": f"W3C {entry['label']} vocabulary · {entry['file']}",
        "url": f"data/w3c/{base}.q42",
        "lexUrl": f"data/w3c/{base}.q42.lex",
        "bidxUrl": f"data/w3c/{base}.q42.bidx",
        "compressed": False,
        "source": "bundled",
        "license": catalog.get("license", "W3C"),
        "homepage": "https://www.w3.org/standards/semanticweb/ontology",
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
    })

manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
manifest["datasets"] = [
    d for d in manifest.get("datasets", []) if d.get("group") != "w3c"
] + built
MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
print(f"manifest: {len(built)} W3C datasets")