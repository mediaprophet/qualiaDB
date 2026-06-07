#!/usr/bin/env python3
"""
Qualia-DB comparative benchmark harness.

Runs engines one at a time to avoid cross-engine overhead bias.
Each runner enforces the 512 MB RAM ceiling; OOM is a valid result.

All results are normalized to a common schema (v1) for easy aggregation
and visualization.

Qualia native daemon (port 4242) is auto-included when --all is used
and the daemon is healthy. Use --no-qualia to disable.

Usage:
    python benchmarks/harness.py --all --output docs/comparative_benchmark_results.json
    python benchmarks/harness.py --engine qualia
    python benchmarks/harness.py --all --no-qualia
"""
import argparse
import datetime
import json
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))

from common import DEFAULT_WARMUP, DEFAULT_SAMPLES, record_dataset_file_metrics
from datasets import list_dataset_profiles, load_dataset_profile
from environment import (
    SCHEMA_VERSION,
    collect_harness_environment,
    fetch_daemon_execution_environment,
    merge_execution_environment,
)

BASE_ENGINES = ["oxigraph", "surrealdb", "comunica", "wasm_prolog", "qualia_wasm"]

ENGINE_META = {
    "oxigraph": {
        "label": "Oxigraph",
        "focus": "Memory bloat & native Rust SPARQL speed",
        "install": "pip install pyoxigraph psutil",
    },
    "surrealdb": {
        "label": "SurrealDB",
        "focus": "Relational-vs-graph overhead (SQL-like document/graph parsing)",
        "install": "surreal CLI binary required",
    },
    "comunica": {
        "label": "Comunica",
        "focus": "WASM/JS execution penalty (high-level runtime overhead)",
        "install": "Node.js + npm install @comunica/query-sparql n3",
    },
    "wasm_prolog": {
        "label": "WASM-Prolog",
        "focus": "Logical inference throughput (backtracking vs. O(1) FNV lookup)",
        "install": "Node.js + npm install tau-prolog",
    },
    "qualia_wasm": {
        "label": "Qualia (WASM)",
        "focus": "Node WASM pipeline — flat quins + Webizen VM execute_ntriples_query",
        "install": "wasm-pack build qualia-core-db → docs/playground/",
    },
    "qualia": {
        "label": "Qualia (native daemon)",
        "focus": "Zero-allocation 5-vector graph + Sentinel VM on your hardware",
        "install": "cargo run --release -p qualia-cli -- daemon --port 4242",
    },
    "qualia_q42": {
        "label": "Qualia (.q42 artifact)",
        "focus": "Pre-compiled SuperBlock load — same graph, no RDF text parse",
        "install": "cargo build --release -p qualia-cli (q42_comparative_bench)",
    },
    "qualia_cq42": {
        "label": "Qualia (.c.q42 artifact)",
        "focus": "LZ4 distribution artifact — decompress + subject index (browser/WebTorrent path)",
        "install": "cargo build --release -p qualia-cli + scripts/prepare_schemaorg_benchmark.ps1 -Compress",
    },
    "qualia_nt": {
        "label": "Qualia (N-Triples parse)",
        "focus": "Same RDF .nt source as Oxigraph/Comunica — parse text into quins in-process",
        "install": "cargo build --release -p qualia-cli (q42_comparative_bench --format ntriples)",
    },
}

QUALIA_FORMAT_ENGINES = ("qualia_nt", "qualia_q42", "qualia_cq42")

METHODOLOGY = (
    f"Each engine is run in complete isolation. 512 MB ceiling enforced. "
    f"Latency: {DEFAULT_WARMUP} warmup + {DEFAULT_SAMPLES} samples (p50/p95/p99). "
    "RDF engines (Oxigraph, SurrealDB, Comunica, WASM-Prolog, Qualia WASM) load N-Triples text. "
    "Schema.org profile adds three Qualia format rows on the same ontology: "
    "N-Triples parse (qualia_nt), native .q42 SuperBlocks (qualia_q42), and .c.q42 LZ4 distribution (qualia_cq42). "
    "Qualia daemon row (when present) measured via local daemon on port 4242. "
    "Qualia WASM uses execute_ntriples_query on flat QualiaQuin bytes in Node."
)


def _qualia_format_engines(dataset_profile: dict) -> list[str]:
    """Qualia rows that compare native formats against the shared RDF source."""
    info = dataset_profile.get("dataset_info") or {}
    engines: list[str] = []
    source_path = dataset_profile.get("source_path") or info.get("source_path")
    if (dataset_profile.get("nt_bytes") or (source_path and os.path.isfile(source_path))):
        engines.append("qualia_nt")
    if info.get("native_q42_available"):
        engines.append("qualia_q42")
    if info.get("compressed_q42_available"):
        engines.append("qualia_cq42")
    return engines


def _qualia_daemon_healthy() -> bool:
    try:
        import urllib.request
        req = urllib.request.Request("http://127.0.0.1:4242/health", method="GET")
        with urllib.request.urlopen(req, timeout=1.0) as r:
            return r.status == 200
    except Exception:
        return False


def normalize_result(engine: str, raw: dict, dataset_profile: dict) -> dict:
    result = dict(raw)
    result.setdefault("engine", engine)
    result.setdefault("schema_version", SCHEMA_VERSION)
    result.setdefault("dataset", dataset_profile["dataset"])
    result.setdefault("dataset_profile", dataset_profile["id"])
    result.setdefault("dataset_label", dataset_profile["label"])
    result.setdefault("n_triples", dataset_profile["n_triples"])
    result.setdefault("timestamp", datetime.datetime.now(datetime.timezone.utc).isoformat().replace("+00:00", "Z"))
    result["meta"] = ENGINE_META.get(engine, {})
    result["dataset_info"] = dict(dataset_profile.get("dataset_info") or {})
    if engine == "qualia":
        result.setdefault("measurement_path", "daemon_http_query")
    elif engine == "qualia_q42":
        result.setdefault("measurement_path", "in_process_q42_superblock")
    elif engine == "qualia_cq42":
        result.setdefault("measurement_path", "in_process_cq42_decompress")
    elif engine == "qualia_nt":
        result.setdefault("measurement_path", "in_process_ntriples_parse")
    elif engine == "comunica":
        result.setdefault("measurement_path", "wasm_js_subprocess")
    elif engine == "wasm_prolog":
        result.setdefault("measurement_path", "wasm_js_subprocess")
    elif engine == "qualia_wasm":
        result.setdefault("measurement_path", "wasm_node_in_process")
    else:
        result.setdefault("measurement_path", "in_process_subprocess")
    for key in ("point", "twohop", "filter"):
        if key not in result:
            result[key] = None
    return record_dataset_file_metrics(result, dataset_profile)


def run_engine(engine: str, n: int, enforce_memory_limit: bool, dataset_profile: dict) -> dict:
    if engine == "oxigraph":
        from oxigraph.runner import benchmark_set
    elif engine == "surrealdb":
        from surrealdb.runner import benchmark_set
    elif engine == "comunica":
        from comunica.runner import benchmark_set
    elif engine == "wasm_prolog":
        from wasm_prolog.runner import benchmark_set
    elif engine == "qualia_wasm":
        from qualia_wasm.runner import benchmark_set
    elif engine == "qualia":
        from qualia.runner import benchmark_set
    elif engine == "qualia_q42":
        from qualia.artifact_runner import benchmark_set_q42 as benchmark_set
    elif engine == "qualia_cq42":
        from qualia.artifact_runner import benchmark_set_cq42 as benchmark_set
    elif engine == "qualia_nt":
        from qualia.artifact_runner import benchmark_set_nt as benchmark_set
    else:
        return {"engine": engine, "error": f"unknown engine: {engine}"}

    raw = benchmark_set(n=n, enforce_memory_limit=enforce_memory_limit, dataset=dataset_profile)
    return normalize_result(engine, raw, dataset_profile)


def merge_into_output(output_path: str, engine: str, result: dict, dataset_profile: dict, daemon_env=None) -> None:
    existing: dict = {}
    if os.path.exists(output_path):
        with open(output_path) as f:
            try:
                existing = json.load(f)
            except json.JSONDecodeError:
                existing = {}

    if "engines" not in existing:
        existing["engines"] = {}
    existing["engines"][engine] = result
    existing["last_updated"] = result.get("timestamp")
    existing["schema_version"] = SCHEMA_VERSION
    existing["dataset"] = dataset_profile["dataset"]
    existing["dataset_profile"] = dataset_profile["id"]
    existing["dataset_label"] = dataset_profile["label"]
    existing["dataset_info"] = dict(dataset_profile.get("dataset_info") or {})
    existing["methodology"] = METHODOLOGY

    if "execution_environment" not in existing:
        existing["execution_environment"] = collect_harness_environment()
    existing["execution_environment"] = merge_execution_environment(
        existing["execution_environment"],
        daemon_env=daemon_env if engine == "qualia" else None,
    )

    with open(output_path, "w") as f:
        json.dump(existing, f, indent=2)
    print(f"[harness] Results merged into {output_path}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Qualia-DB comparative benchmark harness",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""Examples:
  python benchmarks/harness.py --all --output docs/comparative_benchmark_results.json
  python benchmarks/harness.py --all --no-qualia
  python benchmarks/harness.py --engine qualia
"""
    )
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument(
        "--engine",
        choices=BASE_ENGINES + ["qualia"] + list(QUALIA_FORMAT_ENGINES),
        help="Single engine to benchmark",
    )
    group.add_argument("--all", action="store_true", help="Run all engines (Qualia auto-included if daemon healthy)")

    parser.add_argument("--n", type=int, default=10_000, help="Synthetic dataset size in triples")
    parser.add_argument("--output", default=None, help="Write results to this JSON file")
    parser.add_argument("--no-memory-limit", action="store_true", help="Disable 512 MB ceiling (debug only)")
    parser.add_argument("--no-qualia", action="store_true", help="Do not include Qualia even if daemon is running")
    parser.add_argument("--qualia", action="store_true", help="Force include Qualia (even if not auto-detected)")
    parser.add_argument(
        "--dataset-profile",
        choices=list_dataset_profiles(),
        default="synthetic-10k",
        help="Dataset profile to benchmark",
    )

    args = parser.parse_args()

    enforce = not args.no_memory_limit
    dataset_profile = load_dataset_profile(args.dataset_profile, n=args.n)

    if args.engine:
        targets = [args.engine]
    else:
        targets = list(BASE_ENGINES)

        include_qualia = False
        if args.qualia:
            include_qualia = True
        elif not args.no_qualia:
            # Auto-detect
            include_qualia = _qualia_daemon_healthy()

        if include_qualia:
            targets.append("qualia")
            print("[harness] Qualia daemon detected on port 4242 — including native reference", flush=True)

        for fmt_engine in _qualia_format_engines(dataset_profile):
            if fmt_engine not in targets:
                targets.append(fmt_engine)
        if _qualia_format_engines(dataset_profile):
            print(
                "[harness] Qualia format rows: "
                + ", ".join(_qualia_format_engines(dataset_profile)),
                flush=True,
            )

    for engine in targets:
        meta = ENGINE_META.get(engine, {})
        print(
            f"\n[harness] -- {meta.get('label', engine)} "
            f"(profile={dataset_profile['id']}, n={dataset_profile['n_triples']:,}) --",
            flush=True,
        )
        result = run_engine(engine, args.n, enforce, dataset_profile)
        print(json.dumps(result, indent=2), flush=True)

        if args.output:
            daemon_env = fetch_daemon_execution_environment() if engine == "qualia" else None
            merge_into_output(args.output, engine, result, dataset_profile, daemon_env=daemon_env)


if __name__ == "__main__":
    main()
