"""
SurrealDB benchmark runner — not yet implemented.

Planned approach:
  - Spin up `surreal start --memory` as a subprocess
  - Ingest via the SurrealDB HTTP REST API (POST /sql)
  - Measure ingestion, point SELECT, two-hop JOIN, and predicate FETCH
  - Enforce 512 MB with Docker --memory=512m wrapper around the server process

Install (future): surreal CLI binary + surrealdb Python client
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))


def benchmark_set(n: int = 10_000, enforce_memory_limit: bool = True) -> dict:
    return {
        "engine": "surrealdb",
        "n_triples": n,
        "status": "not_implemented",
        "note": "SurrealDB runner pending — see benchmarks/surrealdb/runner.py",
    }
