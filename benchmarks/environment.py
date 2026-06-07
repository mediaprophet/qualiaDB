"""
Benchmark schema v2 — execution environment and device manifest collectors.

Shared by the comparative harness, per-engine runners, and (via JSON shape)
browser live benchmarks.
"""
from __future__ import annotations

import json
import os
import platform
import sys
import urllib.request
from datetime import datetime, timezone
from typing import Any, Optional

SCHEMA_VERSION = 2
MEMORY_CEILING_MB = 512
DAEMON_URL = os.environ.get("QUALIA_DAEMON_URL", "http://127.0.0.1:4242")


def _utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def _host_class() -> str:
    machine = platform.machine().lower()
    system = platform.system().lower()
    if machine in ("arm64", "aarch64"):
        if system == "darwin":
            return "APPLE_SILICON"
        if system in ("linux", "android"):
            # Heuristic: small RAM → mobile class
            ram = _total_ram_gb()
            if ram and ram <= 16:
                return "ARM64_MOBILE"
            return "ARM64_SERVER"
        return "ARM64_SERVER"
    if machine in ("x86_64", "amd64"):
        return "X86_64_SERVER"
    if machine.startswith("wasm") or machine == "unknown":
        return "WASM_BROWSER"
    return "UNKNOWN"


def _total_ram_gb() -> Optional[float]:
    try:
        import psutil
        return round(psutil.virtual_memory().total / (1024 ** 3), 1)
    except ImportError:
        pass
    if sys.platform == "linux":
        try:
            with open("/proc/meminfo") as f:
                for line in f:
                    if line.startswith("MemTotal:"):
                        kb = int(line.split()[1])
                        return round(kb / (1024 ** 2), 1)
        except OSError:
            pass
    return None


def collect_device_manifest() -> dict[str, Any]:
    cores = os.cpu_count()
    return {
        "host_class": _host_class(),
        "cpu_arch": platform.machine(),
        "os": platform.system(),
        "cpu_logical_cores": cores,
        "ram_reported_gb": _total_ram_gb(),
        "has_simd_wasm": False,
        "has_npu": False,
    }


def collect_harness_environment(
    runner: str = "python-comparative-harness",
    measurement_path: str = "harness_isolated_subprocess",
) -> dict[str, Any]:
    return {
        "runner": runner,
        "engine_version": None,
        "memory_ceiling_mb": MEMORY_CEILING_MB,
        "measurement_path": measurement_path,
        "topology": {
            "mode": "serial_harness",
            "worker_cells_configured": 1,
            "worker_cells_active_during_run": 1,
            "compute_swarm_enabled": False,
            "cell_memory_floor_mb": MEMORY_CEILING_MB,
            "scheduling": "serial",
        },
        "device_manifest": collect_device_manifest(),
        "collected_at": _utc_now(),
    }


def fetch_daemon_health(base_url: str = DAEMON_URL, timeout: float = 1.0) -> Optional[dict[str, Any]]:
    try:
        req = urllib.request.Request(f"{base_url.rstrip('/')}/health", method="GET")
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return json.loads(r.read())
    except Exception:
        return None


def fetch_daemon_execution_environment(base_url: str = DAEMON_URL) -> Optional[dict[str, Any]]:
    health = fetch_daemon_health(base_url)
    if not health:
        return None
    env = health.get("execution_environment")
    if isinstance(env, dict):
        merged = dict(env)
        merged.setdefault("device_manifest", collect_device_manifest())
        merged["collected_at"] = _utc_now()
        if health.get("version"):
            merged["engine_version"] = health["version"]
        return merged
    return None


def merge_execution_environment(
    base: dict[str, Any],
    daemon_env: Optional[dict[str, Any]] = None,
) -> dict[str, Any]:
    """Attach harness host manifest; overlay daemon topology when Qualia was measured via daemon."""
    out = dict(base)
    out.setdefault("device_manifest", collect_device_manifest())
    out["collected_at"] = _utc_now()
    if daemon_env:
        topo = daemon_env.get("topology") or {}
        out["qualia_daemon_topology"] = topo
        if daemon_env.get("engine_version"):
            out["engine_version"] = daemon_env["engine_version"]
    return out
