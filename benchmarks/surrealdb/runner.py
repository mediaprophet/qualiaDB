"""
SurrealDB benchmark runner.

Starts `surreal start memory` as a subprocess, ingests the synthetic N-Triples
graph via batch HTTP INSERT statements, then measures point lookup, two-hop
traversal, and predicate filter using SurrealQL.

The 512 MB ceiling is tracked via psutil on the server subprocess (RLIMIT_AS
cannot be applied to a subprocess from Python portably, so we report peak RSS
and let the harness interpret an abnormal exit as OOM).

Install: surreal CLI binary on PATH (see benchmarks.yml for CI install step)
"""
import base64
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
from common import latency_stats_ms, peak_rss_mb, generate_ntriples

WARMUP  = 10
SAMPLES = 50
BATCH   = 500          # triples per INSERT statement
_NT_RE  = re.compile(r'<([^>]+)>\s+<([^>]+)>\s+<([^>]+)>\s+\.')


def _free_port() -> int:
    with socket.socket() as s:
        s.bind(("", 0))
        return s.getsockname()[1]


def _sql(url: str, auth: str, stmt: str) -> list:
    """POST a SurrealQL statement, return parsed JSON result list."""
    req = urllib.request.Request(
        url,
        data=stmt.encode(),
        headers={
            "Accept":       "application/json",
            "Authorization": auth,
            "Surreal-Ns":   "bench",
            "Surreal-Db":   "bench",
            "Content-Type": "text/plain",
        },
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.loads(r.read())


def benchmark_set(n: int = 10_000, enforce_memory_limit: bool = True) -> dict:
    if not shutil.which("surreal"):
        return {
            "engine": "surrealdb",
            "n_triples": n,
            "error": "surreal binary not found — add to PATH (see benchmarks/requirements.txt)",
        }

    port = _free_port()
    url  = f"http://127.0.0.1:{port}/sql"
    auth = "Basic " + base64.b64encode(b"root:root").decode()

    proc = subprocess.Popen(
        ["surreal", "start",
         "--bind", f"127.0.0.1:{port}",
         "--user", "root", "--pass", "root",
         "memory"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    result: dict = {"engine": "surrealdb", "n_triples": n}

    try:
        # Wait for server ready (up to 15 s)
        for _ in range(30):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=1)
                break
            except Exception:
                time.sleep(0.5)
        else:
            result["error"] = "SurrealDB did not become ready within 15 s"
            return result

        # ── Ingestion ─────────────────────────────────────────────────────────
        nt_lines = generate_ntriples(n).decode().strip().split("\n")
        triples  = [_NT_RE.match(ln).groups() for ln in nt_lines]

        rss_before = peak_rss_mb()
        t0 = time.perf_counter()
        try:
            for i in range(0, len(triples), BATCH):
                items = ", ".join(
                    f'{{s: "{s}", p: "{p}", o: "{o}"}}'
                    for s, p, o in triples[i : i + BATCH]
                )
                _sql(url, auth, f"INSERT INTO triple [{items}];")
        except Exception as exc:
            result["ingestion_ms"] = "ERROR"
            result["error"] = str(exc)
            return result

        result["ingestion_ms"]      = round((time.perf_counter() - t0) * 1000, 3)
        result["rss_before_load_mb"] = round(rss_before, 2)
        result["rss_after_load_mb"]  = round(peak_rss_mb(), 2)

        # ── Point lookup ──────────────────────────────────────────────────────
        _POINT = "SELECT * FROM triple WHERE s = 'http://q.test/s/0';"
        try:
            result["point"] = latency_stats_ms(
                lambda: _sql(url, auth, _POINT),
                warmup=WARMUP, samples=SAMPLES,
            )
        except Exception as exc:
            result["point"] = f"ERROR: {exc}"

        # ── Two-hop traversal ─────────────────────────────────────────────────
        _TWOHOP = (
            "LET $b = (SELECT VALUE o FROM triple WHERE s = 'http://q.test/s/0' LIMIT 1)[0];"
            "SELECT * FROM triple WHERE s = $b LIMIT 1;"
        )
        try:
            result["twohop"] = latency_stats_ms(
                lambda: _sql(url, auth, _TWOHOP),
                warmup=WARMUP, samples=SAMPLES,
            )
        except Exception as exc:
            result["twohop"] = f"ERROR: {exc}"

        # ── Predicate filter scan ─────────────────────────────────────────────
        _FILTER = "SELECT * FROM triple WHERE p = 'http://q.test/p/0' LIMIT 100;"
        try:
            result["filter"] = latency_stats_ms(
                lambda: _sql(url, auth, _FILTER),
                warmup=WARMUP, samples=SAMPLES,
            )
        except Exception as exc:
            result["filter"] = f"ERROR: {exc}"

        # RSS of the server subprocess
        try:
            import psutil
            srv_rss = psutil.Process(proc.pid).memory_info().rss / (1024 * 1024)
            result["server_rss_mb"] = round(srv_rss, 2)
        except Exception:
            pass

        result["peak_rss_mb"] = round(peak_rss_mb(), 2)

    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()

    return result


if __name__ == "__main__":
    import json as _json
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 10_000
    print(_json.dumps(benchmark_set(n), indent=2))
