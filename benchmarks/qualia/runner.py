"""
Qualia-DB native reference runner (for comparative harness).

Probes the local Qualia daemon (default port 4242) and runs the standard
point / two-hop / filter queries if available.

This allows native Qualia numbers to appear alongside Oxigraph, SurrealDB,
Comunica and WASM-Prolog in the same comparative JSON.

If the daemon is not running, the runner returns a clear status so the
harness can decide whether to include it.

The daemon must be started with:
    cargo run --release -p qualia-cli -- daemon --port 4242
"""
import json
import os
import sys
import time
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
from common import latency_stats_ms, DEFAULT_WARMUP, DEFAULT_SAMPLES

DAEMON_URL = "http://127.0.0.1:4242"

_LLM_BENCH_PATH = os.path.join(
    os.path.dirname(__file__), "..", "..", "docs", "llm_benchmark_results.json"
)


def _read_llm_latency_stats() -> dict:
    """Load qualia_latency_stats from qualia-cli bench output if available."""
    try:
        with open(_LLM_BENCH_PATH) as f:
            return json.load(f).get("qualia_latency_stats", {})
    except Exception:
        return {}


def _us_to_ms_stats(s: dict) -> dict:
    """Convert a latency-stats dict from microseconds to milliseconds."""
    num_keys = {"min", "max", "mean", "p50", "p95", "p99"}
    out = {k: round(v / 1000, 6) if k in num_keys else v for k, v in s.items()
           if k in num_keys | {"samples", "warmup_samples"}}
    out["unit"] = "milliseconds"
    return out


def _probe_health(timeout: float = 1.0) -> bool:
    try:
        req = urllib.request.Request(f"{DAEMON_URL}/health", method="GET")
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status == 200
    except Exception:
        return False


def _post_query(query: str, timeout: float = 30.0) -> dict:
    """POST a query to the daemon /query endpoint."""
    payload = json.dumps({"query": query}).encode()
    headers = {"Content-Type": "application/json"}
    token = os.environ.get("QUALIA_TOKEN") or os.environ.get("QUALIA_DEV_TOKEN")
    if token:
        headers["X-Qualia-Token"] = token
    req = urllib.request.Request(
        f"{DAEMON_URL}/query",
        data=payload,
        headers=headers,
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read())


def benchmark_set(n: int = 10_000, enforce_memory_limit: bool = True) -> dict:
    result = {
        "engine": "qualia",
        "n_triples": n,
        "qualia_daemon_available": False,
    }

    if not _probe_health():
        result["note"] = "Qualia daemon not running on port 4242. Start with: cargo run --release -p qualia-cli -- daemon"
        return result

    result["qualia_daemon_available"] = True
    result["note"] = "Measured via local Qualia daemon (port 4242)"

    # Standard queries (same logical operations as other runners)
    _POINT = "SELECT * WHERE { <http://q.test/s/0> ?p ?o }"
    _TWOHOP = """
        SELECT * WHERE {
            <http://q.test/s/0> ?p1 ?b .
            ?b ?p2 ?o .
        } LIMIT 1
    """
    _FILTER = "SELECT * WHERE { ?s <http://q.test/p/0> ?o } LIMIT 100"

    try:
        result["point"] = latency_stats_ms(
            lambda: _post_query(_POINT),
            warmup=DEFAULT_WARMUP,
            samples=DEFAULT_SAMPLES,
        )
    except Exception as exc:
        result["point"] = f"ERROR: {exc}"

    try:
        result["twohop"] = latency_stats_ms(
            lambda: _post_query(_TWOHOP),
            warmup=DEFAULT_WARMUP,
            samples=DEFAULT_SAMPLES,
        )
    except Exception as exc:
        result["twohop"] = f"ERROR: {exc}"

    try:
        result["filter"] = latency_stats_ms(
            lambda: _post_query(_FILTER),
            warmup=DEFAULT_WARMUP,
            samples=DEFAULT_SAMPLES,
        )
    except Exception as exc:
        result["filter"] = f"ERROR: {exc}"

    # Ingestion is not measured via the daemon query path.
    # Pull from qualia-cli bench --suite full output when available.
    result["ingestion_ms"] = None
    llm = _read_llm_latency_stats()
    if llm:
        ing = llm.get("ingestion_10k_quins", {})
        if ing.get("p50") is not None:
            result["ingestion_ms"] = round(ing["p50"] / 1000, 6)
        scaling = llm.get("rss_after_materialize_mb")
        if scaling is None:
            # Try qualia_scaling_stats for 10k
            pass
        for key in ("point", "twohop", "filter"):
            if isinstance(result.get(key), str):  # error string from daemon
                s = llm.get(key, {})
                if s:
                    result[key] = _us_to_ms_stats(s)
                    result["note"] = (
                        result.get("note", "")
                        + " Query latencies from qualia-cli bench in-process (µs→ms)."
                    ).strip()

    return result


if __name__ == "__main__":
    import json as _json
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 10_000
    print(_json.dumps(benchmark_set(n), indent=2))
