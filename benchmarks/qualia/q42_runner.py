"""Backward-compatible entry point for qualia_q42 harness runs."""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
from qualia.artifact_runner import benchmark_set_q42 as benchmark_set

if __name__ == "__main__":
    print(json.dumps(benchmark_set(), indent=2))
