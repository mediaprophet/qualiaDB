"""
Comunica benchmark runner.

Spawns a Node.js child process running bench.mjs which loads the synthetic
N-Triples into N3.js and queries via Comunica SPARQL QueryEngine.

512 MB V8 heap enforced via --max-old-space-size.
Uses project-wide DEFAULT_WARMUP/DEFAULT_SAMPLES convention (inner bench.mjs
should align its sample counts for perfect parity).

Install: cd benchmarks/comunica && npm install
"""
import json
import os
import shutil
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
from common import peak_rss_mb, record_dataset_file_metrics, DEFAULT_WARMUP, DEFAULT_SAMPLES

_SCRIPT  = os.path.join(os.path.dirname(__file__), "bench.mjs")
_MODULES = os.path.join(os.path.dirname(__file__), "node_modules")


def _check_deps():
    if not shutil.which("node"):
        return "node not found — install Node.js >= 18"
    if not os.path.isdir(_MODULES):
        return "node_modules not found — run: cd benchmarks/comunica && npm install"
    return None


def benchmark_set(n: int = 10_000, enforce_memory_limit: bool = True, dataset=None) -> dict:
    if dataset is None:
        dataset = {
            "n_triples": n,
            "queries": {
                "point": "SELECT * WHERE { <http://q.test/s/0> ?p ?o }",
                "twohop": "SELECT * WHERE { <http://q.test/s/0> ?p1 ?b . ?b ?p2 ?o . } LIMIT 1",
                "filter": "SELECT * WHERE { ?s <http://q.test/p/0> ?o } LIMIT 100",
            },
        }

    err = _check_deps()
    if err:
        return {"engine": "comunica", "n_triples": dataset.get("n_triples", n), "error": err}

    heap_flag = "--max-old-space-size=512" if enforce_memory_limit else "--max-old-space-size=2048"
    env = dict(os.environ)
    env["QUALIA_BENCH_QUERIES_JSON"] = json.dumps(dataset.get("queries") or {})
    if dataset.get("source_path"):
        env["QUALIA_BENCH_NT_PATH"] = dataset["source_path"]
    else:
        env.pop("QUALIA_BENCH_NT_PATH", None)

    try:
        proc = subprocess.run(
            ["node", heap_flag, _SCRIPT, str(n)],
            capture_output=True, text=True, timeout=300,
            cwd=os.path.dirname(__file__),
            env=env,
        )
    except subprocess.TimeoutExpired:
        return {"engine": "comunica", "n_triples": dataset.get("n_triples", n), "error": "timeout after 300 s"}

    if proc.returncode != 0:
        stderr = proc.stderr.strip()[-500:]
        oom = any(w in stderr.lower() for w in ("heap", "allocation failed", "out of memory"))
        return {
            "engine": "comunica",
            "n_triples": dataset.get("n_triples", n),
            "ingestion_ms": "OOM" if oom else "ERROR",
            "error": f"node exit {proc.returncode}: {stderr}",
        }

    try:
        result = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        return {"engine": "comunica", "n_triples": dataset.get("n_triples", n), "error": f"could not parse bench.mjs output: {exc}\nstdout: {proc.stdout[:300]}"}

    result["peak_rss_mb"] = round(peak_rss_mb(), 2)
    # Record the sample policy used by the harness for traceability
    result.setdefault("_sample_policy", {"warmup": DEFAULT_WARMUP, "samples": DEFAULT_SAMPLES})
    return record_dataset_file_metrics(result, dataset)


if __name__ == "__main__":
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 10_000
    print(json.dumps(benchmark_set(n), indent=2))
