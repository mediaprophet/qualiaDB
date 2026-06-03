"""
Shared utilities for the Qualia-DB comparative benchmark harness.
All runners implement benchmark_set(n) -> dict and use these helpers.
"""
import os
import sys
import time
import statistics


def latency_stats_ms(fn, warmup: int = 10, samples: int = 50) -> dict:
    """Run fn() warmup+samples times, return full statistics in milliseconds."""
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
        "min":     round(times[0], 4),
        "max":     round(times[-1], 4),
        "mean":    round(statistics.mean(times), 4),
        "p50":     round(times[n // 2], 4),
        "p95":     round(times[int(n * 0.95)], 4),
        "p99":     round(times[int(n * 0.99)], 4),
        "samples": samples,
        "warmup_samples": warmup,
        "unit":    "milliseconds",
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
