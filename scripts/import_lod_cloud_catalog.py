#!/usr/bin/env python3
"""
Import reachable datasets from the LOD Cloud registry into QualiaDB's ontology catalog.

Source: https://lod-cloud.net/lod-data.json

Catalog files (repo root):
  resources/catalog.yaml      — master index
  resources/llms.yaml         — LLM / GGUF model catalog
  resources/ontologies.yaml   — ontology catalog (this script writes here)
  resources/sparql_endpoints.yaml — SPARQL endpoint catalog

Usage:
  pip install pyyaml

  # Probe URLs and write a proposed YAML (safe default — does not touch ontologies.yaml)
  python scripts/import_lod_cloud_catalog.py

  # Use a local copy of lod-data.json
  python scripts/import_lod_cloud_catalog.py --input lod-data.json

  # Merge verified new entries into resources/ontologies.yaml
  python scripts/import_lod_cloud_catalog.py --apply

  # Quick test on first 20 datasets
  python scripts/import_lod_cloud_catalog.py --limit 20 --report reports/lod_import.json
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

try:
    import yaml
except ImportError:
    print("PyYAML required: pip install pyyaml", file=sys.stderr)
    sys.exit(1)

LOD_URL = "https://lod-cloud.net/lod-data.json"
REPO_ROOT = Path(__file__).resolve().parents[1]
ONTOLOGIES_PATH = REPO_ROOT / "resources" / "ontologies.yaml"
PROPOSED_PATH = REPO_ROOT / "resources" / "ontologies_lod_proposed.yaml"

# LOD domain → Qualia catalog domain tag
DOMAIN_MAP: dict[str, str] = {
    "linguistics": "linguistics",
    "life sciences": "health",
    "health": "health",
    "geography": "geography",
    "government": "government",
    "media": "media",
    "publications": "publications",
    "user-generated content": "social",
    "crossdomain": "general",
    "cross-domain": "general",
    "social web": "social",
    "economics": "economics",
    "legal": "legal",
    "education": "education",
    "science": "science",
    "technology": "technology",
}

# Keyword hints → human-readable use cases (stored in notes / tags)
USE_CASE_HINTS: dict[str, str] = {
    "lod": "linked open data integration",
    "llod": "linguistic linked open data",
    "lexical-resources": "lexical / terminology resources",
    "linguistics": "language and lexicon modelling",
    "format-foaf": "social graph / identity (FOAF)",
    "format-sioc": "online community structures (SIOC)",
    "format-dc": "Dublin Core metadata",
    "format-skos": "concept schemes and taxonomies (SKOS)",
    "format-owl": "OWL ontologies and TBox reasoning",
    "format-rdf": "general RDF knowledge graphs",
    "provenance": "provenance and attribution",
    "geographic": "geospatial linked data",
    "biology": "life-science entities and relations",
    "chemistry": "chemical structures and properties",
    "government": "government open data",
    "crossdomain": "general-purpose reference data",
}

MEDIA_TO_FORMAT: dict[str, str] = {
    "application/x-ntriples": "ntriples",
    "application/n-triples": "ntriples",
    "text/plain": "ntriples",
    "text/turtle": "ttl",
    "application/x-turtle": "ttl",
    "application/rdf+xml": "rdf",
    "text/rdf": "rdf",
    "application/owl+xml": "owl",
    "application/ld+json": "jsonld",
    "application/json": "jsonld",
}


@dataclass
class ProbeResult:
    url: str
    ok: bool
    status: str
    kind: str  # download | sparql | website | example


@dataclass
class DatasetReport:
    id: str
    title: str
    reachable: bool
    skipped_reason: str | None = None
    probes: list[ProbeResult] = field(default_factory=list)
    catalog_entry: dict[str, Any] | None = None


def slug_id(identifier: str) -> str:
    s = identifier.strip().lower()
    s = re.sub(r"[^a-z0-9]+", "-", s)
    return s.strip("-")[:64] or "lod-dataset"


def normalize_license(raw: str | None) -> str | None:
    if not raw:
        return None
    low = raw.lower()
    if "cc-by-sa" in low or "creativecommons.org/licenses/by-sa" in low:
        return "CC-BY-SA"
    if "cc-by" in low or "creativecommons.org/licenses/by" in low:
        return "CC-BY"
    if "cc0" in low or "publicdomain" in low:
        return "CC0"
    if "odc-by" in low:
        return "ODC-BY"
    if "w3c" in low:
        return "W3C"
    if "opendatacommons" in low:
        return "ODC"
    return raw.split("/")[-1][:40] or None


def media_to_format(media_type: str | None, url: str) -> str:
    if media_type:
        mt = media_type.split(";")[0].strip().lower()
        if mt in MEDIA_TO_FORMAT:
            return MEDIA_TO_FORMAT[mt]
    path = urlparse(url).path.lower()
    for ext, fmt in (
        (".ttl", "ttl"),
        (".owl", "owl"),
        (".rdf", "rdf"),
        (".jsonld", "jsonld"),
        (".json-ld", "jsonld"),
        (".nt", "ntriples"),
        (".ntriples", "ntriples"),
        (".n3", "n3"),
        (".gz", "ntriples"),  # common for LOD dumps
    ):
        if path.endswith(ext):
            return fmt
    return "rdf"


def estimate_size_mb(triples: Any) -> float | None:
    if triples is None:
        return None
    if isinstance(triples, (int, float)):
        n = int(triples)
    else:
        s = str(triples).strip().replace(",", "")
        if not s.isdigit():
            return None
        n = int(s)
    if n <= 0:
        return None
    # rough: ~80 bytes per triple on disk compressed-ish
    return max(0.1, round(n * 80 / (1024 * 1024), 2))


def categorize(entry: dict[str, Any]) -> tuple[str, list[str], str]:
    """Return (domain, tags, use_case_notes)."""
    domain_raw = (entry.get("domain") or "general").strip().lower()
    domain = DOMAIN_MAP.get(domain_raw, domain_raw.replace(" ", "-")[:32])

    keywords = [k.lower() for k in (entry.get("keywords") or [])]
    tags = sorted({domain, "lod-cloud", *keywords[:8]})

    use_cases: list[str] = []
    for kw in keywords:
        if kw in USE_CASE_HINTS:
            use_cases.append(USE_CASE_HINTS[kw])
    if domain == "health":
        use_cases.append("clinical / biomedical knowledge graphs")
    elif domain == "linguistics":
        use_cases.append("NLP, lexicons, and wordnets")
    elif domain == "geography":
        use_cases.append("geospatial queries and gazetteers")
    elif domain == "government":
        use_cases.append("policy, statistics, and civic open data")

    desc = ""
    d = entry.get("description") or {}
    if isinstance(d, dict):
        desc = (d.get("en") or next(iter(d.values()), "")).strip()
    elif isinstance(d, str):
        desc = d.strip()

    if desc and not use_cases:
        use_cases.append("general RDF dataset")

    notes_parts = []
    if use_cases:
        notes_parts.append("Useful for: " + "; ".join(dict.fromkeys(use_cases)))
    if desc:
        short = desc[:280] + ("..." if len(desc) > 280 else "")
        notes_parts.append(short)

    return domain, tags, " ".join(notes_parts)


def lod_status_ok(status: str | None) -> bool:
    if not status:
        return False
    return status.strip().upper().startswith("OK")


def probe_url(url: str, kind: str, timeout: float) -> ProbeResult:
    if not url or not url.startswith(("http://", "https://")):
        return ProbeResult(url=url or "", ok=False, status="invalid", kind=kind)
    req = urllib.request.Request(
        url,
        method="HEAD",
        headers={"User-Agent": "QualiaDB-LOD-Importer/1.0"},
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            code = resp.status
            ok = 200 <= code < 400
            return ProbeResult(url=url, ok=ok, status=str(code), kind=kind)
    except urllib.error.HTTPError as e:
        # Some servers reject HEAD; try GET with Range
        if e.code in (403, 405, 501):
            return _probe_get(url, kind, timeout)
        return ProbeResult(url=url, ok=False, status=f"HTTP {e.code}", kind=kind)
    except Exception as e:
        return ProbeResult(url=url, ok=False, status=type(e).__name__, kind=kind)


def _probe_get(url: str, kind: str, timeout: float) -> ProbeResult:
    req = urllib.request.Request(
        url,
        method="GET",
        headers={"User-Agent": "QualiaDB-LOD-Importer/1.0", "Range": "bytes=0-0"},
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            code = resp.status
            ok = 200 <= code < 400
            return ProbeResult(url=url, ok=ok, status=str(code), kind=kind)
    except Exception as e:
        return ProbeResult(url=url, ok=False, status=str(e)[:80], kind=kind)


def collect_candidate_urls(entry: dict[str, Any]) -> list[tuple[str, str, str | None]]:
    """(url, kind, media_type) candidates in priority order."""
    out: list[tuple[str, str, str | None]] = []

    for block in entry.get("full_download") or []:
        url = block.get("download_url")
        if url:
            out.append((url, "download", block.get("media_type")))

    for block in entry.get("other_download") or []:
        url = block.get("access_url") or block.get("download_url")
        if url:
            out.append((url, "download", block.get("media_type")))

    for block in entry.get("sparql") or []:
        url = block.get("access_url")
        if url:
            out.append((url, "sparql", None))

    website = entry.get("website")
    if website:
        out.append((website, "website", None))

    return out


def pick_best_download(
    entry: dict[str, Any], probes: list[ProbeResult]
) -> tuple[str | None, str | None]:
    """Choose best reachable download URL and media type."""
    ok_downloads = {p.url for p in probes if p.ok and p.kind == "download"}
    for block in entry.get("full_download") or []:
        url = block.get("download_url")
        if url and url in ok_downloads:
            return url, block.get("media_type")
    for block in entry.get("other_download") or []:
        url = block.get("access_url") or block.get("download_url")
        if url and url in ok_downloads:
            return url, block.get("media_type")
    # Fall back to reachable website if no dump
    for p in probes:
        if p.ok and p.kind == "website":
            return p.url, "text/html"
    return None, None


def build_catalog_entry(
    entry: dict[str, Any],
    download_url: str,
    media_type: str | None,
    existing_ids: set[str],
) -> dict[str, Any]:
    identifier = entry.get("identifier") or entry.get("_id") or entry.get("title", "lod")
    oid = slug_id(str(identifier))
    base_id = oid
    n = 2
    while oid in existing_ids:
        oid = f"{base_id}-{n}"
        n += 1

    domain, tags, notes = categorize(entry)
    fmt = media_to_format(media_type, download_url)
    size = estimate_size_mb(entry.get("triples"))
    title = (entry.get("title") or identifier).strip()
    license_ = normalize_license(entry.get("license"))

    item: dict[str, Any] = {
        "id": oid,
        "name": title,
        "acronym": identifier if identifier != title else None,
        "source": "LOD Cloud",
        "format": fmt,
        "download": {"type": "direct", "url": download_url},
        "license": license_,
        "domain": domain,
        "tags": tags,
        "last_verified": date.today().isoformat(),
        "notes": notes,
        "import_strategy": "modular" if size and size > 50 else None,
    }
    if size is not None:
        item["size_estimate_mb"] = size

    # Drop None values for cleaner YAML
    return {k: v for k, v in item.items() if v is not None}


def load_lod_data(path: Path | None, url: str) -> dict[str, Any]:
    if path:
        raw = path.read_text(encoding="utf-8")
    else:
        with urllib.request.urlopen(url, timeout=60) as resp:
            raw = resp.read().decode("utf-8")
    # Cursor / browser saves may prepend metadata lines before the JSON object
    start = raw.find("{")
    if start < 0:
        raise ValueError("No JSON object found in LOD data")
    return json.loads(raw[start:])


def load_existing_ontology_ids(path: Path) -> set[str]:
    if not path.is_file():
        return set()
    data = yaml.safe_load(path.read_text(encoding="utf-8")) or {}
    return {item["id"] for item in (data.get("ontologies") or []) if item.get("id")}


def load_existing_download_urls(path: Path) -> set[str]:
    if not path.is_file():
        return set()
    data = yaml.safe_load(path.read_text(encoding="utf-8")) or {}
    urls: set[str] = set()
    for item in data.get("ontologies") or []:
        dl = item.get("download") or {}
        if dl.get("url"):
            urls.add(dl["url"])
    return urls


def process_dataset(
    key: str,
    entry: dict[str, Any],
    *,
    timeout: float,
    skip_probe: bool,
    require_download: bool,
    existing_ids: set[str],
    existing_urls: set[str],
) -> DatasetReport:
    title = (entry.get("title") or key).strip()
    report = DatasetReport(id=slug_id(key), title=title, reachable=False)

    candidates = collect_candidate_urls(entry)
    if not candidates:
        report.skipped_reason = "no URLs"
        return report

    # Fast path: trust LOD Cloud pre-computed status when skipping probe
    if skip_probe:
        for block in entry.get("full_download") or []:
            if lod_status_ok(block.get("status")) and block.get("download_url"):
                url = block["download_url"]
                if url in existing_urls:
                    report.skipped_reason = "duplicate URL"
                    return report
                report.reachable = True
                report.probes.append(
                    ProbeResult(url=url, ok=True, status="OK (lod-data)", kind="download")
                )
                report.catalog_entry = build_catalog_entry(
                    entry, url, block.get("media_type"), existing_ids
                )
                return report
        report.skipped_reason = "no OK full_download in lod-data"
        return report

    probes: list[ProbeResult] = []
    for url, kind, _mt in candidates[:6]:  # cap probes per dataset
        probes.append(probe_url(url, kind, timeout))
    report.probes = probes

    if not any(p.ok for p in probes):
        report.skipped_reason = "no reachable URLs"
        return report

    download_url, media_type = pick_best_download(entry, probes)
    if require_download and not download_url:
        # SPARQL-only datasets are not ontology file imports
        report.skipped_reason = "sparql-only (no dump)"
        return report

    if not download_url:
        report.skipped_reason = "no download URL"
        return report

    low = download_url.lower()
    if require_download and "sparql" in low and not low.endswith((".ttl", ".owl", ".rdf", ".nt", ".gz")):
        report.skipped_reason = "sparql endpoint only"
        return report

    if download_url in existing_urls:
        report.skipped_reason = "duplicate URL"
        return report

    report.reachable = True
    report.catalog_entry = build_catalog_entry(
        entry, download_url, media_type, existing_ids
    )
    if report.catalog_entry:
        existing_ids.add(report.catalog_entry["id"])
        existing_urls.add(download_url)
    return report


def write_proposed_yaml(path: Path, entries: list[dict[str, Any]]) -> None:
    doc = {
        "ontologies": entries,
        "_meta": {
            "source": LOD_URL,
            "generated": date.today().isoformat(),
            "note": "Review before merging into ontologies.yaml",
        },
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        yaml.dump(doc, sort_keys=False, allow_unicode=True, default_flow_style=False),
        encoding="utf-8",
    )


def merge_into_catalog(catalog_path: Path, new_entries: list[dict[str, Any]]) -> int:
    data = yaml.safe_load(catalog_path.read_text(encoding="utf-8")) or {}
    existing = data.get("ontologies") or []
    existing_ids = {e["id"] for e in existing if e.get("id")}
    added = 0
    for entry in new_entries:
        if entry["id"] not in existing_ids:
            existing.append(entry)
            existing_ids.add(entry["id"])
            added += 1
    data["ontologies"] = existing
    catalog_path.write_text(
        yaml.dump(data, sort_keys=False, allow_unicode=True, default_flow_style=False),
        encoding="utf-8",
    )
    return added


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, help="Local lod-data.json (skip network fetch)")
    parser.add_argument("--url", default=LOD_URL, help="LOD Cloud JSON URL")
    parser.add_argument(
        "--output",
        type=Path,
        default=PROPOSED_PATH,
        help="Write proposed ontology entries here",
    )
    parser.add_argument(
        "--apply",
        action="store_true",
        help="Merge new entries into resources/ontologies.yaml",
    )
    parser.add_argument("--limit", type=int, default=0, help="Process at most N datasets (0 = all)")
    parser.add_argument("--timeout", type=float, default=12.0, help="HTTP probe timeout (seconds)")
    parser.add_argument("--workers", type=int, default=8, help="Parallel dataset workers")
    parser.add_argument(
        "--skip-probe",
        action="store_true",
        help="Trust lod-data.json status field (OK) instead of live HTTP probes",
    )
    parser.add_argument(
        "--sparql-only",
        action="store_true",
        help="Also import SPARQL-only datasets (endpoint URL as download url)",
    )
    parser.add_argument("--report", type=Path, help="Write JSON summary report")
    args = parser.parse_args()

    require_download = not args.sparql_only

    print(f"Loading LOD data from {'file' if args.input else 'network'}...")
    lod = load_lod_data(args.input, args.url)
    items = list(lod.items())
    if args.limit:
        items = items[: args.limit]
    print(f"Processing {len(items)} datasets...")

    existing_ids = load_existing_ontology_ids(ONTOLOGIES_PATH)
    existing_urls = load_existing_download_urls(ONTOLOGIES_PATH)

    reports: list[DatasetReport] = []
    new_entries: list[dict[str, Any]] = []

    def task(item: tuple[str, dict]) -> DatasetReport:
        key, entry = item
        return process_dataset(
            key,
            entry,
            timeout=args.timeout,
            skip_probe=args.skip_probe,
            require_download=require_download,
            existing_ids=existing_ids,
            existing_urls=existing_urls,
        )

    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        futures = {pool.submit(task, item): item[0] for item in items}
        for i, fut in enumerate(as_completed(futures), 1):
            rep = fut.result()
            reports.append(rep)
            if rep.catalog_entry:
                new_entries.append(rep.catalog_entry)
            if i % 25 == 0 or i == len(items):
                ok = sum(1 for r in reports if r.reachable)
                print(f"  ... {i}/{len(items)} checked, {ok} reachable so far")

    # Dedupe (parallel workers may overlap) and stable sort
    seen_ids: set[str] = set()
    seen_urls: set[str] = set()
    deduped: list[dict[str, Any]] = []
    for entry in sorted(new_entries, key=lambda e: e["id"]):
        url = (entry.get("download") or {}).get("url", "")
        if entry["id"] in seen_ids or url in seen_urls:
            continue
        seen_ids.add(entry["id"])
        if url:
            seen_urls.add(url)
        deduped.append(entry)
    new_entries = deduped

    write_proposed_yaml(args.output, new_entries)
    print(f"Wrote {len(new_entries)} proposed entries -> {args.output}")

    reachable = [r for r in reports if r.reachable]
    skipped = [r for r in reports if r.skipped_reason]
    print(f"Summary: {len(reachable)} reachable, {len(skipped)} skipped, {len(items)} total")

    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(
            json.dumps(
                {
                    "generated": date.today().isoformat(),
                    "total": len(items),
                    "reachable": len(reachable),
                    "added_to_proposed": len(new_entries),
                    "skipped_reasons": {
                        reason: sum(1 for r in skipped if r.skipped_reason == reason)
                        for reason in sorted({r.skipped_reason for r in skipped if r.skipped_reason})
                    },
                    "datasets": [
                        {
                            "id": r.id,
                            "title": r.title,
                            "reachable": r.reachable,
                            "skipped_reason": r.skipped_reason,
                            "probes": [p.__dict__ for p in r.probes],
                            "catalog_id": (r.catalog_entry or {}).get("id"),
                        }
                        for r in sorted(reports, key=lambda x: x.id)
                    ],
                },
                indent=2,
            ),
            encoding="utf-8",
        )
        print(f"Report -> {args.report}")

    if args.apply:
        if not new_entries:
            print("Nothing to apply.")
            return 0
        added = merge_into_catalog(ONTOLOGIES_PATH, new_entries)
        print(f"Merged {added} new entries into {ONTOLOGIES_PATH}")
    else:
        print("Dry run complete. Use --apply to merge into resources/ontologies.yaml")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
