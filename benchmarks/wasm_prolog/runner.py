"""
WASM-Prolog (tau-prolog) benchmark runner.

Spawns a Node.js child process running bench.js (CJS), which consults the
synthetic graph as Prolog facts and measures backtracking-based search latency.
The 512 MB V8 heap ceiling is enforced via --max-old-space-size=512.

The key architectural finding: Prolog's depth-first backtracking search over
10k facts vs. Qualia-DB's O(1) FNV-indexed hash lookup for the same queries.
OOM during consult at 512 MB is a valid and expected result for large datasets.

Install: Node.js >= 18, then:
    cd benchmarks/wasm_prolog && npm install
"""
import json
import os
import shutil
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
from common import peak_rss_mb

_SCRIPT  = os.path.join(os.path.dirname(__file__), "bench.js")
_MODULES = os.path.join(os.path.dirname(__file__), "node_modules")


def _check_deps():
    if not shutil.which("node"):
        return "node not found — install Node.js >= 18"
    if not os.path.isdir(_MODULES):
        return "node_modules not found — run:  cd benchmarks/wasm_prolog && npm install"
    return None


def benchmark_set(n: int = 10_000, enforce_memory_limit: bool = True) -> dict:
    err = _check_deps()
    if err:
        return {"engine": "wasm_prolog", "n_triples": n, "error": err}

    heap_flag = "--max-old-space-size=512" if enforce_memory_limit else "--max-old-space-size=2048"

    try:
        proc = subprocess.run(
            ["node", heap_flag, _SCRIPT, str(n)],
            capture_output=True,
            text=True,
            timeout=300,
            cwd=os.path.dirname(__file__),
        )
    except subprocess.TimeoutExpired:
        return {"engine": "wasm_prolog", "n_triples": n, "error": "timeout after 300 s"}

    if proc.returncode != 0:
        stderr = proc.stderr.strip()[-500:]
        oom = any(w in stderr.lower() for w in ("heap", "allocation failed", "out of memory"))
        return {
            "engine":      "wasm_prolog",
            "n_triples":   n,
            "ingestion_ms": "OOM" if oom else "ERROR",
            "error": f"node exit {proc.returncode}: {stderr}",
        }

    try:
        result = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        return {
            "engine": "wasm_prolog", "n_triples": n,
            "error": f"could not parse bench.js output: {exc}\nstdout: {proc.stdout[:300]}",
        }

    result["peak_rss_mb"] = round(peak_rss_mb(), 2)
    return result


if __name__ == "__main__":
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 10_000
    print(json.dumps(benchmark_set(n), indent=2))
