"""
Dataset profiles for the comparative benchmark harness.

Profiles let the harness benchmark either the built-in synthetic graph or a
real external ontology snapshot such as Schema.org 30.0. Each profile provides
enough metadata for the runners and docs pages to describe how the results were
produced, and for RDF-based runners to reuse the same point/two-hop/filter
queries.
"""
from __future__ import annotations

import os
import re
from collections import Counter
from typing import Any, Optional

from common import file_size_mb, generate_ntriples

SCHEMAORG_RELEASE = "30.0"
SCHEMAORG_VARIANT = "current-https"
SCHEMAORG_SOURCE_FILE = f"schemaorg-{SCHEMAORG_VARIANT}.nt"
SCHEMAORG_SOURCE_URL = (
    "https://raw.githubusercontent.com/schemaorg/schemaorg/main/"
    f"data/releases/{SCHEMAORG_RELEASE}/{SCHEMAORG_SOURCE_FILE}"
)

_NT_LINE_RE = re.compile(r"^\s*<([^>]+)>\s+<([^>]+)>\s+(.+?)\s+\.\s*$")
_IRI_OBJECT_RE = re.compile(r"^<([^>]+)>$")


def _workspace_root() -> str:
    return os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))


def _schemaorg_source_path(root: str) -> str:
    return os.path.join(root, "data", "schemaorg", SCHEMAORG_RELEASE, SCHEMAORG_SOURCE_FILE)


def _schemaorg_q42_base_path(root: str) -> str:
    return os.path.join(root, "data", "schemaorg", SCHEMAORG_RELEASE, f"schemaorg-{SCHEMAORG_VARIANT}")


def _synthetic_profile(n: int) -> dict[str, Any]:
    nt_bytes = generate_ntriples(n)
    nt_mb = round(len(nt_bytes) / (1024 * 1024), 3)
    return {
        "id": "synthetic-10k",
        "label": "Synthetic 10K triples",
        "dataset": "synthetic-ntriples-v1",
        "kind": "synthetic",
        "source_format": "generated-ntriples",
        "n_triples": n,
        "nt_bytes": nt_bytes,
        "queries": {
            "point_subject": "http://q.test/s/0",
            "twohop_start": "http://q.test/s/0",
            "filter_predicate": "http://q.test/p/0",
            "point": "SELECT * WHERE { <http://q.test/s/0> ?p ?o }",
            "twohop": (
                "SELECT * WHERE {\n"
                "    <http://q.test/s/0> ?p1 ?b .\n"
                "    ?b ?p2 ?o .\n"
                "} LIMIT 1"
            ),
            "filter": "SELECT * WHERE { ?s <http://q.test/p/0> ?o } LIMIT 100",
        },
        "dataset_info": {
            "label": "Synthetic 10K triples",
            "release": None,
            "variant": None,
            "source_format": "generated-ntriples",
            "source_url": None,
            "source_path": None,
            "native_format": None,
            "native_q42_base_path": None,
            "native_q42_path": None,
            "native_q42_available": False,
            "compressed_q42_path": None,
            "compressed_q42_available": False,
            "n_triples": n,
            "source_file_mb": nt_mb,
            "source_file_bytes": len(nt_bytes),
        },
    }


def _scan_external_nt(nt_bytes: bytes) -> tuple[dict[str, str], int]:
    text = nt_bytes.decode("utf-8")
    first_subject = None
    subjects: set[str] = set()
    iri_edges: list[tuple[str, str]] = []
    predicate_counts: Counter[str] = Counter()
    triple_count = 0

    for line in text.splitlines():
        match = _NT_LINE_RE.match(line)
        if not match:
            continue

        subject, predicate, raw_object = match.groups()
        triple_count += 1
        first_subject = first_subject or subject
        subjects.add(subject)
        predicate_counts[predicate] += 1

        iri_obj = _IRI_OBJECT_RE.match(raw_object)
        if iri_obj:
            iri_edges.append((subject, iri_obj.group(1)))

    if not first_subject:
        raise ValueError("dataset did not contain parseable N-Triples")

    twohop_start = first_subject
    for subject, obj in iri_edges:
        if obj in subjects:
            twohop_start = subject
            break

    filter_predicate = predicate_counts.most_common(1)[0][0] if predicate_counts else "http://www.w3.org/2000/01/rdf-schema#comment"

    return {
        "point_subject": first_subject,
        "twohop_start": twohop_start,
        "filter_predicate": filter_predicate,
        "point": f"SELECT * WHERE {{ <{first_subject}> ?p ?o }} LIMIT 100",
        "twohop": (
            "SELECT * WHERE {\n"
            f"    <{twohop_start}> ?p1 ?b .\n"
            "    ?b ?p2 ?o .\n"
            "} LIMIT 1"
        ),
        "filter": f"SELECT * WHERE {{ ?s <{filter_predicate}> ?o }} LIMIT 100",
    }, triple_count


def _schemaorg_profile(root: str) -> dict[str, Any]:
    source_path = _schemaorg_source_path(root)
    q42_base = _schemaorg_q42_base_path(root)
    q42_path = f"{q42_base}.q42"
    compressed_q42_path = f"{q42_base}.c.q42"

    dataset_info = {
        "label": "Schema.org 30.0 current HTTPS",
        "release": SCHEMAORG_RELEASE,
        "variant": SCHEMAORG_VARIANT,
        "source_format": "ntriples",
        "source_url": SCHEMAORG_SOURCE_URL,
        "source_path": source_path,
        "native_format": "q42",
        "native_q42_base_path": q42_base,
        "native_q42_path": q42_path,
        "native_q42_available": os.path.exists(q42_path),
        "compressed_q42_path": compressed_q42_path,
        "compressed_q42_available": os.path.exists(compressed_q42_path),
        "n_triples": 0,
    }

    profile = {
        "id": "schemaorg-30-current-https",
        "label": "Schema.org 30.0 current HTTPS",
        "dataset": "schemaorg-30-current-https",
        "kind": "external-ntriples",
        "source_format": "ntriples",
        "source_path": source_path,
        "source_url": SCHEMAORG_SOURCE_URL,
        "nt_bytes": None,
        "queries": None,
        "n_triples": 0,
        "dataset_info": dataset_info,
    }

    if not os.path.exists(source_path):
        dataset_info["missing_source"] = True
        return profile

    with open(source_path, "rb") as f:
        nt_bytes = f.read()

    queries, triple_count = _scan_external_nt(nt_bytes)
    dataset_info["n_triples"] = triple_count
    dataset_info["missing_source"] = False
    dataset_info["source_file_mb"] = file_size_mb(source_path)
    dataset_info["source_file_bytes"] = len(nt_bytes)
    if os.path.exists(q42_path):
        dataset_info["native_q42_file_mb"] = file_size_mb(q42_path)
        dataset_info["native_q42_file_bytes"] = os.path.getsize(q42_path)
    if os.path.exists(compressed_q42_path):
        dataset_info["compressed_q42_file_mb"] = file_size_mb(compressed_q42_path)
        dataset_info["compressed_q42_file_bytes"] = os.path.getsize(compressed_q42_path)
    profile["nt_bytes"] = nt_bytes
    profile["queries"] = queries
    profile["n_triples"] = triple_count
    return profile


def load_dataset_profile(name: str, n: int = 10_000, root: Optional[str] = None) -> dict[str, Any]:
    root = os.path.abspath(root or _workspace_root())
    if name == "synthetic-10k":
        return _synthetic_profile(n)
    if name == "schemaorg-30-current-https":
        return _schemaorg_profile(root)
    raise ValueError(f"unknown dataset profile: {name}")


def list_dataset_profiles() -> list[str]:
    return ["synthetic-10k", "schemaorg-30-current-https"]
