#!/usr/bin/env python3
"""
Qualia-DB comparative benchmark harness.

Runs engines one at a time to avoid cross-engine overhead bias.
Each runner enforces the 512 MB RAM ceiling; OOM is a valid result.

Usage:
    python benchmarks/harness.py --engine oxigraph
    python benchmarks/harness.py --engine oxigraph --n 100000
    python benchmarks/harness.py --engine oxigraph --output docs/comparative_benchmark_results.json
    python benchmarks/harness.py --all --output docs/comparative_benchmark_results.json
"""
import argparse
import datetime
import json
import os
import sys

# Ensure the benchmarks/ directory is on the path regardless of cwd
sys.path.insert(0, os.path.dirname(__file__))

ENGINES = ["oxigraph", "surrealdb", "comunica", "wasm_prolog"]

ENGINE_META = {
    "oxigraph": {
        "label":   "Oxigraph",
        "focus":   "Memory bloat & native Rust SPARQL speed",
        "install": "pip install pyoxigraph psutil",
    },
    "surrealdb": {
        "label":   "SurrealDB",
        "focus":   "Relational-vs-graph overhead (SQL-like document/graph parsing)",
        "install": "surreal CLI binary required",
    },
    "comunica": {
        "label":   "Comunica",
        "focus":   "WASM/JS execution penalty (high-level runtime overhead)",
        "install": "Node.js + npm install @comunica/query-sparql n3",
    },
    "wasm_prolog": {
        "label":   "WASM-Prolog",
        "focus":   "Logical inference throughput (backtracking vs. O(1) FNV lookup)",
        "install": "Node.js + npm install tau-prolog",
    },
}


def run_engine(engine: str, n: int, enforce_memory_limit: bool) -> dict:
    if engine == "oxigraph":
        from oxigraph.runner import benchmark_set
    elif engine == "surrealdb":
        from surrealdb.runner import benchmark_set
    elif engine == "comunica":
        from comunica.runner import benchmark_set
    elif engine == "wasm_prolog":
        from wasm_prolog.runner import benchmark_set
    else:
        return {"engine": engine, "error": f"unknown engine: {engine}"}

    result = benchmark_set(n=n, enforce_memory_limit=enforce_memory_limit)
    result.setdefault("engine", engine)
    result["meta"] = ENGINE_META.get(engine, {})
    result["timestamp"] = datetime.datetime.now(datetime.timezone.utc).isoformat().replace("+00:00", "Z")
    return result


def merge_into_output(output_path: str, engine: str, result: dict) -> None:
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
    existing["last_updated"] = result["timestamp"]
    existing["methodology"] = (
        "Each engine is run in isolation (one at a time) to avoid cross-engine "
        "overhead bias.  The 512 MB RAM ceiling is enforced via Linux RLIMIT_AS; "
        "OOM during ingestion or query execution is a valid benchmark result."
    )

    with open(output_path, "w") as f:
        json.dump(existing, f, indent=2)
    print(f"[harness] Results merged into {output_path}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser(description="Qualia-DB comparative benchmark harness")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--engine", choices=ENGINES, help="Single engine to benchmark")
    group.add_argument("--all", action="store_true", help="Run all implemented engines sequentially")
    parser.add_argument("--n", type=int, default=10_000,
                        help="Synthetic dataset size in triples (default: 10000)")
    parser.add_argument("--output", default=None,
                        help="Merge JSON results into this file (e.g. docs/comparative_benchmark_results.json)")
    parser.add_argument("--no-memory-limit", action="store_true",
                        help="Disable the 512 MB ceiling (for debugging)")
    args = parser.parse_args()

    enforce = not args.no_memory_limit
    targets = ENGINES if args.all else [args.engine]

    for engine in targets:
        print(f"\n[harness] -- {ENGINE_META[engine]['label']} (n={args.n:,}) --", flush=True)
        result = run_engine(engine, args.n, enforce)

        # Pretty-print the result
        print(json.dumps(result, indent=2), flush=True)

        if args.output:
            merge_into_output(args.output, engine, result)


if __name__ == "__main__":
    main()
