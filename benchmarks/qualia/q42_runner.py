"""
Qualia .q42 artifact runner for the comparative harness.

Loads a pre-built `.q42` file (Schema.org profile) via the
`q42_comparative_bench` binary and reports the same point / two-hop / filter
metrics as the RDF-based engines.
"""
import json
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
from common import record_dataset_file_metrics, peak_rss_mb

_REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
_BENCH_BIN = os.path.join(_REPO_ROOT, "target", "release", "q42_comparative_bench.exe")
_BENCH_BIN_UNIX = os.path.join(_REPO_ROOT, "target", "release", "q42_comparative_bench")


def _bench_executable() -> str:
    if os.path.isfile(_BENCH_BIN):
        return _BENCH_BIN
    if os.path.isfile(_BENCH_BIN_UNIX):
        return _BENCH_BIN_UNIX
    return "q42_comparative_bench"


def benchmark_set(n: int = 10_000, enforce_memory_limit: bool = True, dataset=None) -> dict:
    if dataset is None:
        dataset = {"n_triples": n, "queries": {}, "dataset_info": {}}

    dataset_info = dataset.get("dataset_info") or {}
    q42_path = dataset_info.get("native_q42_path")
    result = {
        "engine": "qualia_q42",
        "n_triples": dataset.get("n_triples", n),
        "measurement_path": "in_process_q42_mmap",
    }

    if not q42_path or not os.path.isfile(q42_path):
        result["error"] = (
            "native .q42 artifact missing — run scripts/prepare_schemaorg_benchmark.ps1 "
            "-Release 30.0 -Variant current-https -Compress"
        )
        record_dataset_file_metrics(result, dataset)
        return result

    queries = dataset.get("queries") or {}
    needed = ("point_subject", "twohop_start", "filter_predicate")
    if not all(queries.get(k) for k in needed):
        result["error"] = "dataset profile missing query anchors for .q42 benchmark"
        record_dataset_file_metrics(result, dataset)
        return result

    query_payload = json.dumps({k: queries[k] for k in needed})
    exe = _bench_executable()

    try:
        proc = subprocess.run(
            [exe, "--q42", q42_path, "--queries-json", query_payload],
            capture_output=True,
            text=True,
            timeout=600,
            cwd=_REPO_ROOT,
        )
    except subprocess.TimeoutExpired:
        result["error"] = "q42_comparative_bench timed out after 600 s"
        record_dataset_file_metrics(result, dataset)
        return result
    except FileNotFoundError:
        result["error"] = (
            "q42_comparative_bench not built — run: cargo build --release -p qualia-cli"
        )
        record_dataset_file_metrics(result, dataset)
        return result

    if proc.returncode != 0:
        stderr = (proc.stderr or proc.stdout or "").strip()[-500:]
        result["error"] = f"q42_comparative_bench exit {proc.returncode}: {stderr}"
        record_dataset_file_metrics(result, dataset)
        return result

    try:
        result = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        result["error"] = f"could not parse q42 bench output: {exc}"
        record_dataset_file_metrics(result, dataset)
        return result

    result["peak_rss_mb"] = round(peak_rss_mb(), 2)
    if dataset_info.get("compressed_q42_available"):
        result["note"] = (
            result.get("note", "")
            + f" Distribution artifact: {dataset_info.get('compressed_q42_path')} "
            f"({dataset_info.get('compressed_q42_file_mb', '?')} MB)."
        ).strip()

    record_dataset_file_metrics(result, dataset)
    return result


if __name__ == "__main__":
    print(json.dumps(benchmark_set(), indent=2))
