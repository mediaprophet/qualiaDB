//! STELLAR §A AH-track H1(a) cache — the **hardware passport** (decision D26).
//!
//! Probing the host (H0 topology + H1(a) cross-circuit benchmark) costs real time; doing it every
//! boot burns TTFT (A7). The passport caches the discovered topology + capability matrix to a small
//! **CBOR** blob keyed by the host's **adapter identifiers** (vendor:device handles — identifiers,
//! not an "identity"). On later boots, if the current adapter set matches the cached key, the probe
//! is skipped; a topology change invalidates the cache and forces a re-probe (D26/D28).
//!
//! CBOR (via `ciborium`, serde-compatible) is used rather than JSON: it round-trips IEEE-754 floats
//! natively — including the `f64::INFINITY` that marks an in-pool (no-transfer) circuit — and a
//! compact binary blob fits the project's `.q42` binary-first ethos. **Cache only (no signing):** the
//! human-key *signing* of the passport (H1(b)) is gated on the identity remediation and lives
//! elsewhere; this module never claims trust, only fast-boot.
//!
//! Native only.
#![cfg(not(target_arch = "wasm32"))]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::device_benchmark::{benchmark_devices, CapabilityMatrix};
use crate::host_topology::{probe_host_topology, HostTopology};

/// Bump when the passport layout changes (older blobs are then ignored → re-probe).
pub const PASSPORT_VERSION: u32 = 1;

/// Default representative GEMV side length for the cached benchmark.
pub const PASSPORT_GEMV_N: usize = 2048;

/// Cached discovery: topology + measured capability matrix, keyed by adapter identifiers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwarePassport {
    pub version: u32,
    /// Stable key from the discovered adapter identifiers (sorted `vendor:device` handles).
    pub key: String,
    pub topology: HostTopology,
    pub matrix: CapabilityMatrix,
}

/// Stable key from the discovered adapter identifiers (sorted `vendor:device`). Identifiers (handles),
/// never an identity — this is referential integrity for "is this the same hardware set", nothing more.
pub fn topology_key(topo: &HostTopology) -> String {
    let mut ids: Vec<String> = topo
        .adapters
        .iter()
        .map(|a| format!("{:04x}:{:04x}", a.vendor, a.device))
        .collect();
    ids.sort();
    ids.join(",")
}

/// Default cache location (OS temp dir). Callers may pass any path (e.g. a per-user data dir).
pub fn default_cache_path() -> PathBuf {
    std::env::temp_dir().join("qualia_hardware_passport.cbor")
}

/// Serialize a passport to a CBOR blob on disk.
pub fn write_passport(passport: &HardwarePassport, path: &Path) -> Result<(), String> {
    let mut buf = Vec::new();
    ciborium::into_writer(passport, &mut buf).map_err(|e| format!("cbor encode: {e}"))?;
    std::fs::write(path, &buf).map_err(|e| format!("write {}: {e}", path.display()))
}

/// Read + decode a passport (CBOR). Returns `None` on missing file, decode error, or version mismatch.
pub fn read_passport(path: &Path) -> Option<HardwarePassport> {
    let bytes = std::fs::read(path).ok()?;
    let passport: HardwarePassport = ciborium::from_reader(&bytes[..]).ok()?;
    if passport.version != PASSPORT_VERSION {
        return None;
    }
    Some(passport)
}

/// Fast-boot entry: return the cached passport iff its adapter-identifier key matches the current
/// host; otherwise probe (H0 + H1(a) benchmark), cache, and return. `(passport, was_cached)`.
pub fn load_or_probe(path: &Path, gemv_n: usize) -> (HardwarePassport, bool) {
    let topology = probe_host_topology();
    let current_key = topology_key(&topology);

    if let Some(cached) = read_passport(path) {
        if cached.key == current_key {
            return (cached, true); // fast-boot: skip the probe (TTFT)
        }
        // key mismatch ⇒ hardware changed (D26/D28) → re-probe below.
    }

    let matrix = benchmark_devices(gemv_n);
    let fresh = HardwarePassport {
        version: PASSPORT_VERSION,
        key: current_key,
        topology,
        matrix,
    };
    let _ = write_passport(&fresh, path);
    (fresh, false)
}

/// Convenience: fast-boot against the default cache path + GEMV size.
pub fn load_or_probe_default() -> (HardwarePassport, bool) {
    load_or_probe(&default_cache_path(), PASSPORT_GEMV_N)
}

/// Best measured GPU backend name from a cached passport, if any (`"Dx12"`, `"Vulkan"`, `"Metal"`…).
/// Does **not** re-probe. Used by `gpu_context` when `QUALIA_WGPU_BACKEND` is unset so the
/// machine's measured ranking (not a static Windows→DX12 rule alone) selects the path.
pub fn cached_preferred_wgpu_backend() -> Option<String> {
    let path = default_cache_path();
    let passport = read_passport(&path)?;
    let best = passport.matrix.best()?;
    // CPU-only win → do not pin a GPU backend.
    if matches!(best.kind, crate::device_benchmark::CircuitKind::Cpu) {
        return None;
    }
    Some(best.backend.clone())
}

/// Map a passport `backend` string (`"Dx12"`, `"Vulkan"`, …) to a `QUALIA_WGPU_BACKEND` env value.
pub fn backend_env_token(backend: &str) -> Option<&'static str> {
    let s = backend.to_ascii_lowercase();
    if s.contains("dx12") || s.contains("d3d12") {
        Some("dx12")
    } else if s.contains("vulkan") {
        Some("vulkan")
    } else if s.contains("metal") {
        Some("metal")
    } else if s.contains("gl") {
        Some("gl")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_benchmark::{CapabilityMatrix, CircuitBench, CircuitKind};
    use crate::host_topology::{AdapterClass, AdapterDesc, HostMemoryTopology, HostTopology};

    const GB: u64 = 1024 * 1024 * 1024;

    fn synthetic_passport() -> HardwarePassport {
        let topology = HostTopology {
            adapters: vec![
                AdapterDesc {
                    name: "Discrete GPU".into(),
                    backend: "Dx12".into(),
                    class: AdapterClass::Discrete,
                    vendor: 0x10de,
                    device: 0x2571,
                    dedicated_vram_bytes: 12 * GB,
                },
                AdapterDesc {
                    name: "iGPU".into(),
                    backend: "Dx12".into(),
                    class: AdapterClass::Integrated,
                    vendor: 0x8086,
                    device: 0x1912,
                    dedicated_vram_bytes: 0,
                },
            ],
            topology: HostMemoryTopology::Discrete,
            host_ram_bytes: 64 * GB,
            host_ram_available_bytes: 32 * GB,
            cpu_cores: 8,
            os_floor_bytes: 3 * GB / 2,
            usable_model_budget_bytes: 12 * GB,
        };
        let matrix = CapabilityMatrix {
            circuits: vec![
                CircuitBench {
                    label: "Discrete GPU".into(),
                    kind: CircuitKind::DiscreteGpu,
                    backend: "Dx12".into(),
                    ms_per_gemv: 0.43,
                    gflops: 19.5,
                    upload_gbps: 3.3,
                    rel_score: 1.0,
                },
                CircuitBench {
                    label: "CPU native".into(),
                    kind: CircuitKind::Cpu,
                    backend: "native".into(),
                    ms_per_gemv: 23.0,
                    gflops: 0.4,
                    upload_gbps: f64::INFINITY, // in-pool — must survive the round-trip
                    rel_score: 0.02,
                },
            ],
            gemv_n: 2048,
            npu_probed: false,
        };
        HardwarePassport {
            version: PASSPORT_VERSION,
            key: topology_key(&topology),
            topology,
            matrix,
        }
    }

    #[test]
    fn cbor_round_trip_preserves_infinity_and_key() {
        let p = synthetic_passport();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("passport.cbor");

        write_passport(&p, &path).expect("write");
        let back = read_passport(&path).expect("read");

        assert_eq!(back.version, PASSPORT_VERSION);
        assert_eq!(back.key, p.key);
        assert_eq!(back.key, "10de:2571,8086:1912");
        assert_eq!(back.topology.adapters.len(), 2);
        // The in-pool sentinel (infinite bandwidth) must survive CBOR — the reason for CBOR over JSON.
        let cpu = back
            .matrix
            .circuits
            .iter()
            .find(|c| c.kind == CircuitKind::Cpu)
            .unwrap();
        assert!(
            cpu.upload_gbps.is_infinite(),
            "f64::INFINITY must round-trip through CBOR"
        );
        assert!((back.matrix.circuits[0].ms_per_gemv - 0.43).abs() < 1e-9);
    }

    #[test]
    fn version_mismatch_is_ignored() {
        let mut p = synthetic_passport();
        p.version = 999;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("passport.cbor");
        write_passport(&p, &path).unwrap();
        assert!(
            read_passport(&path).is_none(),
            "stale version must be rejected → re-probe"
        );
    }

    /// Real fast-boot path: first call probes + caches (was_cached=false); second loads (true).
    /// Runs the benchmark once (CPU always available); GPU rows appear if an adapter is present.
    #[test]
    fn load_or_probe_caches_then_hits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("passport.cbor");

        let (first, cached1) = load_or_probe(&path, 512);
        assert!(!cached1, "first call must probe");
        assert!(path.exists(), "probe must write the cache");
        assert!(!first.matrix.circuits.is_empty());

        let (second, cached2) = load_or_probe(&path, 512);
        assert!(cached2, "second call must hit the cache (fast-boot)");
        assert_eq!(second.key, first.key);
    }
}
