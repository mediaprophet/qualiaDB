"""
Qualia core WASM benchmark runner (Node.js + qualia_core_db wasm-pack artifacts).

Spawns bench.mjs which encodes synthetic N-Triples as flat QualiaQuin bytes
and queries via execute_ntriples_query.

Requires docs/playground/qualia_core_db.{js,_bg.wasm} (built by wasm-pack on CI).

Install: Node.js >= 18; WASM artifacts from `wasm-pack build` or Pages CI.
"""
import json
import os
import shutil
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
from common import peak_rss_mb, record_dataset_file_metrics, DEFAULT_WARMUP, DEFAULT_SAMPLES

_SCRIPT = os.path.join(os.path.dirname(__file__), "bench.mjs")
_REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
_PLAYGROUND = os.path.join(_REPO_ROOT, "docs", "playground")
_WASM_FILE = os.path.join(_PLAYGROUND, "qualia_core_db_bg.wasm")
_JS_FILE = os.path.join(_PLAYGROUND, "qualia_core_db.js")


def _check_deps():
    if not shutil.which("node"):
        return "node not found — install Node.js >= 18"
    if not os.path.isfile(_WASM_FILE) or not os.path.isfile(_JS_FILE):
        return (
            "qualia_core_db WASM not built — run wasm-pack build on qualia-core-db "
            "or wait for CI to populate docs/playground/"
        )
    return None


def benchmark_set(n: int = 10_000, enforce_memory_limit: bool = True, dataset=None) -> dict:
    if dataset is None:
        dataset = {
            "n_triples": n,
            "queries": {
                "point_subject": "http://q.test/s/0",
                "twohop_start": "http://q.test/s/0",
                "filter_predicate": "http://q.test/p/0",
            },
        }

    err = _check_deps()
    if err:
        return {"engine": "qualia_wasm", "n_triples": dataset.get("n_triples", n), "error": err}

    heap_flag = "--max-old-space-size=512" if enforce_memory_limit else "--max-old-space-size=2048"
    env = dict(os.environ)
    env["QUALIA_BENCH_QUERIES_JSON"] = json.dumps(dataset.get("queries") or {})
    env["QUALIA_WASM_PLAYGROUND"] = _PLAYGROUND
    if dataset.get("source_path"):
        env["QUALIA_BENCH_NT_PATH"] = dataset["source_path"]
    else:
        env.pop("QUALIA_BENCH_NT_PATH", None)

    try:
        proc = subprocess.run(
            ["node", heap_flag, _SCRIPT, str(dataset.get("n_triples", n))],
            capture_output=True,
            text=True,
            timeout=300,
            cwd=os.path.dirname(__file__),
            env=env,
        )
    except subprocess.TimeoutExpired:
        return {"engine": "qualia_wasm", "n_triples": dataset.get("n_triples", n), "error": "timeout after 300 s"}

    if proc.returncode != 0:
        stderr = (proc.stderr or "").strip()[-500:]
        oom = any(w in stderr.lower() for w in ("heap", "allocation failed", "out of memory"))
        return {
            "engine": "qualia_wasm",
            "n_triples": dataset.get("n_triples", n),
            "ingestion_ms": "OOM" if oom else "ERROR",
            "error": f"node exit {proc.returncode}: {stderr}",
        }

    try:
        result = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        return {
            "engine": "qualia_wasm",
            "n_triples": dataset.get("n_triples", n),
            "error": f"could not parse bench.mjs output: {exc}\nstdout: {proc.stdout[:300]}",
        }

    binary_rss = result.get("peak_rss_mb")
    if binary_rss is None or binary_rss <= 0:
        measured = round(peak_rss_mb(), 2)
        if measured > 0:
            result["peak_rss_mb"] = measured
    result.setdefault(
        "_sample_policy",
        {"warmup": DEFAULT_WARMUP, "samples": DEFAULT_SAMPLES},
    )
    return record_dataset_file_metrics(result, dataset)


if __name__ == "__main__":
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 10_000
    print(json.dumps(benchmark_set(n), indent=2))
