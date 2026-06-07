"""
Shared utilities for the Qualia-DB comparative benchmark harness.
All runners implement benchmark_set(n) -> dict and use these helpers.

Standardized defaults balance statistical quality with practicality for slower engines.
"""
import os
import sys
import time
import statistics
from typing import Any, Dict, Optional


# Standardized defaults (used by all runners unless explicitly overridden)
DEFAULT_WARMUP = 10
DEFAULT_SAMPLES = 30          # Good p99 stability; slow engines (Prolog) may use fewer in their runner


def latency_stats_ms(fn, warmup: int = None, samples: int = None) -> dict:
    """Run fn() warmup + samples times and return rich latency statistics (ms).

    Falls back to DEFAULT_WARMUP / DEFAULT_SAMPLES when args are None.
    Always includes the actual sample counts used in the returned dict.
    """
    if warmup is None:
        warmup = DEFAULT_WARMUP
    if samples is None:
        samples = DEFAULT_SAMPLES

    for _ in range(warmup):
        fn()
    times = []
    for _ in range(samples):
        t0 = time.perf_counter()
        fn()
        times.append((time.perf_counter() - t0) * 1000.0)
    times.sort()
    n = len(times)
    return {
        "min":            round(times[0], 4),
        "max":            round(times[-1], 4),
        "mean":           round(statistics.mean(times), 4),
        "p50":            round(times[n // 2], 4),
        "p95":            round(times[int(n * 0.95)], 4),
        "p99":            round(times[int(n * 0.99)], 4),
        "samples":        samples,
        "warmup_samples": warmup,
        "unit":           "milliseconds",
    }


def peak_rss_mb() -> float:
    """Current process RSS in MB. Uses psutil; falls back to /proc/self/status."""
    try:
        import psutil
        return psutil.Process(os.getpid()).memory_info().rss / (1024 * 1024)
    except ImportError:
        pass
    try:
        with open("/proc/self/status") as f:
            for line in f:
                if line.startswith("VmRSS:"):
                    return int(line.split()[1]) / 1024
    except OSError:
        pass
    return 0.0


def apply_512mb_limit():
    """
    Impose a 512 MB virtual-address-space ceiling on the current process (Linux only).
    Any allocation beyond 512 MB raises MemoryError, which runners catch and report as OOM.
    This enforces the edge-compute floor used as Qualia-DB's design target.
    """
    if sys.platform != "linux":
        return  # cgroups/rlimit only meaningful on Linux; skip on macOS/Windows
    try:
        import resource
        limit = 512 * 1024 * 1024
        resource.setrlimit(resource.RLIMIT_AS, (limit, limit))
    except Exception:
        pass  # best-effort; CI may restrict setrlimit


def file_size_bytes(path: Optional[str]) -> Optional[int]:
    if not path or not os.path.isfile(path):
        return None
    return os.path.getsize(path)


def file_size_mb(path: Optional[str]) -> Optional[float]:
    nbytes = file_size_bytes(path)
    if nbytes is None:
        return None
    return round(nbytes / (1024 * 1024), 3)


def record_dataset_file_metrics(result: Dict[str, Any], dataset: Optional[Dict[str, Any]]) -> Dict[str, Any]:
    """
    Attach on-disk dataset artifact sizes so comparative tables can show
    what each engine actually loaded (NT source vs native .q42).
    """
    if dataset is None:
        return result

    dataset_info = dataset.get("dataset_info") or {}
    nt_bytes = dataset.get("nt_bytes")
    source_path = dataset.get("source_path") or dataset_info.get("source_path")

    if result.get("dataset_file_mb") is None and nt_bytes is not None:
        result["dataset_file_bytes"] = len(nt_bytes)
        result["dataset_file_mb"] = round(len(nt_bytes) / (1024 * 1024), 3)
        result["dataset_format"] = dataset.get("source_format") or dataset_info.get("source_format") or "ntriples"
    elif result.get("dataset_file_mb") is None and source_path:
        nbytes = file_size_bytes(source_path)
        if nbytes is not None:
            result["dataset_file_bytes"] = nbytes
            result["dataset_file_mb"] = round(nbytes / (1024 * 1024), 3)
            result["dataset_format"] = dataset.get("source_format") or dataset_info.get("source_format") or "ntriples"
            result["dataset_file_path"] = source_path

    q42_path = dataset_info.get("native_q42_path")
    if q42_path and os.path.exists(q42_path):
        result["native_q42_file_mb"] = file_size_mb(q42_path)
    c_path = dataset_info.get("compressed_q42_path")
    if c_path and os.path.exists(c_path):
        result["compressed_q42_file_mb"] = file_size_mb(c_path)

    return result


def generate_ntriples(n: int) -> bytes:
    """
    Deterministic synthetic N-Triples dataset matching the Qualia CLI bench structure.
    Subject i has predicate (i % 5) and points to object ((i * 13) % n).
    Produces a bipartite-ish graph with 5 distinct predicate types.
    """
    lines = []
    for i in range(n):
        p_idx = i % 5
        o_idx = (i * 13) % n
        lines.append(
            f"<http://q.test/s/{i}> <http://q.test/p/{p_idx}> <http://q.test/o/{o_idx}> .\n"
        )
    return "".join(lines).encode()
