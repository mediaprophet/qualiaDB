"""
Oxigraph benchmark runner.

Measures ingestion, point lookup, two-hop traversal, and predicate filter
using pyoxigraph (in-memory store) against the standard synthetic N-Triples
dataset.  Enforces the 512 MB RAM ceiling so OOM is a reportable result.

Uses the standardized DEFAULT_WARMUP / DEFAULT_SAMPLES from common.py.

Install: pip install pyoxigraph psutil
"""
import os
import sys
import time

# Allow running as a standalone script from the repo root
sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
from common import (
    latency_stats_ms,
    peak_rss_mb,
    apply_512mb_limit,
    generate_ntriples,
    record_dataset_file_metrics,
    DEFAULT_WARMUP,
    DEFAULT_SAMPLES,
)

# SPARQL queries — same logical operations as the Qualia CLI bench
# Subject 0 is the anchor; predicate 0 is the high-frequency lane (n/5 triples).
_POINT_QUERY = "SELECT * WHERE { <http://q.test/s/0> ?p ?o }"

_TWOHOP_QUERY = """
SELECT * WHERE {
    <http://q.test/s/0> ?p1 ?b .
    ?b ?p2 ?o .
} LIMIT 1
"""

_FILTER_QUERY = "SELECT * WHERE { ?s <http://q.test/p/0> ?o } LIMIT 100"


def benchmark_set(n: int = 10_000, enforce_memory_limit: bool = True, dataset=None) -> dict:
    """
    Run the full Oxigraph benchmark suite.

    Args:
        n:                    Number of triples in the synthetic dataset.
        enforce_memory_limit: If True, apply the 512 MB ceiling before ingestion.

    Returns:
        dict with ingestion_ms, point/twohop/filter latency stats, and peak RSS.
        Latency stats now use the project-wide DEFAULT_WARMUP/DEFAULT_SAMPLES.
    """
    if dataset is None:
        dataset = {
            "n_triples": n,
            "nt_bytes": generate_ntriples(n),
            "queries": {
                "point": _POINT_QUERY,
                "twohop": _TWOHOP_QUERY,
                "filter": _FILTER_QUERY,
            },
        }

    result: dict = {"engine": "oxigraph", "n_triples": dataset.get("n_triples", n)}

    try:
        import pyoxigraph
    except ImportError:
        result["error"] = "pyoxigraph not installed — run: pip install pyoxigraph psutil"
        return result

    if enforce_memory_limit:
        apply_512mb_limit()

    # ── Ingestion ─────────────────────────────────────────────────────────────────────
    nt_bytes = dataset.get("nt_bytes")
    if not nt_bytes:
        result["ingestion_ms"] = "ERROR"
        result["error"] = f"dataset source not available for profile {dataset.get('id', 'unknown')}"
        return result

    queries = dataset.get("queries") or {}
    point_query = queries.get("point", _POINT_QUERY)
    twohop_query = queries.get("twohop", _TWOHOP_QUERY)
    filter_query = queries.get("filter", _FILTER_QUERY)

    rss_before = peak_rss_mb()
    t0 = time.perf_counter()
    try:
        store = pyoxigraph.Store()
        from io import BytesIO
        buf = BytesIO(nt_bytes)
        # API changed in pyoxigraph 0.3: bulk_loader() context manager removed;
        # replaced by store.bulk_load(input, mime_type) direct method.
        if hasattr(store, 'bulk_load'):
            store.bulk_load(buf, "application/n-triples")
        elif hasattr(store, 'bulk_loader'):
            loader = store.bulk_loader()
            loader.load_n_triples(buf)
        else:
            store.load(buf, "application/n-triples")
        result["ingestion_ms"]       = round((time.perf_counter() - t0) * 1000, 3)
        result["rss_before_load_mb"] = round(rss_before, 2)
        result["rss_after_load_mb"]  = round(peak_rss_mb(), 2)
    except MemoryError:
        result["ingestion_ms"] = "OOM"
        result["error"] = "OOM during ingestion — 512 MB ceiling reached"
        return result
    except Exception as exc:
        result["ingestion_ms"] = "ERROR"
        result["error"] = str(exc)
        return result

    # ── Point lookup ─────────────────────────────────────────────────────────────────────
    try:
        result["point"] = latency_stats_ms(
            lambda: list(store.query(point_query)),
            warmup=DEFAULT_WARMUP, samples=DEFAULT_SAMPLES,
        )
    except MemoryError:
        result["point"] = "OOM"
    except Exception as exc:
        result["point"] = f"ERROR: {exc}"

    # ── Two-hop traversal ─────────────────────────────────────────────────────────────────────
    try:
        result["twohop"] = latency_stats_ms(
            lambda: list(store.query(twohop_query)),
            warmup=DEFAULT_WARMUP, samples=DEFAULT_SAMPLES,
        )
    except MemoryError:
        result["twohop"] = "OOM"
    except Exception as exc:
        result["twohop"] = f"ERROR: {exc}"

    # ── Predicate filter scan ─────────────────────────────────────────────────────────────────
    try:
        result["filter"] = latency_stats_ms(
            lambda: list(store.query(filter_query)),
            warmup=DEFAULT_WARMUP, samples=DEFAULT_SAMPLES,
        )
    except MemoryError:
        result["filter"] = "OOM"
    except Exception as exc:
        result["filter"] = f"ERROR: {exc}"

    result["peak_rss_mb"] = round(peak_rss_mb(), 2)
    return record_dataset_file_metrics(result, dataset)


if __name__ == "__main__":
    import json
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 10_000
    print(json.dumps(benchmark_set(n), indent=2))
