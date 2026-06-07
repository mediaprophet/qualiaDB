"""
Qualia format runners for the comparative harness (Schema.org profile).

Three in-process rows via q42_comparative_bench:
  qualia_nt   — N-Triples parse (same RDF text as Oxigraph / Comunica / WASM-Prolog)
  qualia_q42  — native SuperBlock .q42 artifact
  qualia_cq42 — LZ4 .c.q42 distribution artifact
"""
import json
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
from common import peak_rss_mb, record_dataset_file_metrics

_REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
_BENCH_BIN = os.path.join(_REPO_ROOT, "target", "release", "q42_comparative_bench.exe")
_BENCH_BIN_UNIX = os.path.join(_REPO_ROOT, "target", "release", "q42_comparative_bench")

_PREP_CMD = (
    "scripts/prepare_schemaorg_benchmark.ps1 -Release 30.0 -Variant current-https -Compress"
)


def _bench_executable() -> str:
    if os.path.isfile(_BENCH_BIN):
        return _BENCH_BIN
    if os.path.isfile(_BENCH_BIN_UNIX):
        return _BENCH_BIN_UNIX
    return "q42_comparative_bench"


def _run_bench(engine: str, input_path: str, fmt: str, dataset: dict) -> dict:
    result = {
        "engine": engine,
        "n_triples": dataset.get("n_triples", 0),
    }

    if not input_path or not os.path.isfile(input_path):
        result["error"] = (
            f"{fmt} input missing ({input_path or 'unset'}) — run: powershell -ExecutionPolicy Bypass -File "
            + _PREP_CMD
        )
        return record_dataset_file_metrics(result, dataset)

    queries = dataset.get("queries") or {}
    needed = ("point_subject", "twohop_start", "filter_predicate")
    if not all(queries.get(k) for k in needed):
        result["error"] = "dataset profile missing query anchors for Qualia artifact benchmark"
        return record_dataset_file_metrics(result, dataset)

    query_payload = json.dumps({k: queries[k] for k in needed})
    exe = _bench_executable()

    try:
        proc = subprocess.run(
            [
                exe,
                "--input",
                input_path,
                "--format",
                fmt,
                "--engine",
                engine,
                "--queries-json",
                query_payload,
            ],
            capture_output=True,
            text=True,
            timeout=600,
            cwd=_REPO_ROOT,
        )
    except subprocess.TimeoutExpired:
        result["error"] = "q42_comparative_bench timed out after 600 s"
        return record_dataset_file_metrics(result, dataset)
    except FileNotFoundError:
        result["error"] = (
            "q42_comparative_bench not built — run: cargo build --release -p qualia-cli"
        )
        return record_dataset_file_metrics(result, dataset)

    if proc.returncode != 0:
        stderr = (proc.stderr or proc.stdout or "").strip()[-500:]
        result["error"] = f"q42_comparative_bench exit {proc.returncode}: {stderr}"
        return record_dataset_file_metrics(result, dataset)

    try:
        result = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        result["error"] = f"could not parse bench output: {exc}"
        return record_dataset_file_metrics(result, dataset)

    binary_rss = result.get("peak_rss_mb")
    if binary_rss is None or binary_rss <= 0:
        measured = round(peak_rss_mb(), 2)
        if measured > 0:
            result["peak_rss_mb"] = measured

    return record_dataset_file_metrics(result, dataset)


def benchmark_set_nt(n: int = 10_000, enforce_memory_limit: bool = True, dataset=None) -> dict:
    if dataset is None:
        dataset = {"n_triples": n, "queries": {}, "dataset_info": {}}
    info = dataset.get("dataset_info") or {}
    nt_path = dataset.get("source_path") or info.get("source_path")
    return _run_bench("qualia_nt", nt_path, "ntriples", dataset)


def benchmark_set_q42(n: int = 10_000, enforce_memory_limit: bool = True, dataset=None) -> dict:
    if dataset is None:
        dataset = {"n_triples": n, "queries": {}, "dataset_info": {}}
    info = dataset.get("dataset_info") or {}
    q42_path = info.get("native_q42_path")
    result = _run_bench("qualia_q42", q42_path, "superblock", dataset)
    if info.get("compressed_q42_available") and not result.get("error"):
        result["note"] = (
            result.get("note", "")
            + f" Distribution artifact: {info.get('compressed_q42_path')} "
            f"({info.get('compressed_q42_file_mb', '?')} MB)."
        ).strip()
    return result


def benchmark_set_cq42(n: int = 10_000, enforce_memory_limit: bool = True, dataset=None) -> dict:
    if dataset is None:
        dataset = {"n_triples": n, "queries": {}, "dataset_info": {}}
    info = dataset.get("dataset_info") or {}
    cq42_path = info.get("compressed_q42_path")
    return _run_bench("qualia_cq42", cq42_path, "cq42", dataset)
