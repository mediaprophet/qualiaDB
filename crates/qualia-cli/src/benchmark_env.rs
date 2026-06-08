//! Execution-environment metadata for benchmark JSON exports (schema v2).

use serde_json::json;

const MEMORY_CEILING_MB: u16 = 512;

fn host_class() -> &'static str {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        return "APPLE_SILICON";
    }
    #[cfg(all(target_arch = "aarch64", any(target_os = "android", target_os = "ios")))]
    {
        return "ARM64_MOBILE";
    }
    #[cfg(all(
        target_arch = "aarch64",
        not(any(target_os = "macos", target_os = "android", target_os = "ios"))
    ))]
    {
        return "ARM64_SERVER";
    }
    #[cfg(target_arch = "x86_64")]
    {
        return "X86_64_SERVER";
    }
    #[cfg(target_arch = "wasm32")]
    {
        return "WASM_BROWSER";
    }
    #[cfg(not(any(
        all(target_arch = "aarch64", target_os = "macos"),
        all(target_arch = "aarch64", any(target_os = "android", target_os = "ios")),
        all(
            target_arch = "aarch64",
            not(any(target_os = "macos", target_os = "android", target_os = "ios"))
        ),
        target_arch = "x86_64",
        target_arch = "wasm32",
    )))]
    {
        return "UNKNOWN";
    }
}

/// Non-identifying device/host fingerprint for cross-machine benchmark cohorts.
pub fn collect_device_manifest() -> serde_json::Value {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_memory();
    let ram_gb = (sys.total_memory() as f64) / (1024.0 * 1024.0 * 1024.0);

    json!({
        "host_class": host_class(),
        "cpu_arch": std::env::consts::ARCH,
        "os": std::env::consts::OS,
        "cpu_logical_cores": sysinfo::System::physical_core_count(),
        "ram_reported_gb": (ram_gb * 10.0).round() / 10.0,
        "has_simd_wasm": false,
        "has_npu": false
    })
}

/// Environment block for `qualia-cli bench --suite full` (single-process microbench).
pub fn bench_execution_environment() -> serde_json::Value {
    json!({
        "runner": "qualia-cli bench",
        "engine_version": env!("CARGO_PKG_VERSION"),
        "memory_ceiling_mb": MEMORY_CEILING_MB,
        "measurement_path": "in_process_microbench",
        "topology": {
            "mode": "single_process",
            "worker_cells_configured": 1,
            "worker_cells_active_during_run": 1,
            "compute_swarm_enabled": false,
            "cell_memory_floor_mb": MEMORY_CEILING_MB,
            "scheduling": "serial"
        },
        "device_manifest": collect_device_manifest()
    })
}
