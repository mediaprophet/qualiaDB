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
use std::path::PathBuf;

use crate::device_benchmark::{benchmark_devices, CapabilityMatrix, CircuitKind};
use crate::host_topology::{probe_host_topology, HostTopology};
use std::path::Path;

/// Bump when the passport layout changes (older blobs are then ignored → re-probe).
/// v2: optional `decode_proxy_tok_s` per circuit + ranking by real decode when measured.
pub const PASSPORT_VERSION: u32 = 2;

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
    /// Best GPU backend token for inference (`dx12` / `vulkan` / `metal` / `gl`), if any.
    /// Derived from the measured matrix; used when `QUALIA_WGPU_BACKEND` is unset.
    #[serde(default)]
    pub preferred_inference_backend: Option<String>,
    /// GEMV n used for the matrix (for operator honesty).
    #[serde(default)]
    pub probe_gemv_n: usize,
    /// Model path used for decode-proxy ranking (if any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decode_proxy_model: Option<String>,
    /// Decode tokens used for the proxy (0 = none).
    #[serde(default)]
    pub decode_proxy_tokens: u32,
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
    let preferred = matrix
        .best()
        .and_then(|c| backend_env_token(&c.backend))
        .map(str::to_string);
    let fresh = HardwarePassport {
        version: PASSPORT_VERSION,
        key: current_key,
        topology,
        matrix,
        preferred_inference_backend: preferred,
        probe_gemv_n: gemv_n,
        decode_proxy_model: None,
        decode_proxy_tokens: 0,
    };
    let _ = write_passport(&fresh, path);
    (fresh, false)
}

/// Default small-model candidates for decode-proxy ranking (first existing wins).
pub fn default_decode_proxy_model() -> Option<std::path::PathBuf> {
    if let Ok(p) = std::env::var("QUALIA_LLM_PROFILE_MODEL") {
        let pb = std::path::PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    const CANDIDATES: &[&str] = &[
        r"C:\LLM_Models\P64\smollm2-360m-instruct-q8_0.f16.p64",
        r"C:\LLM_Models\P64\smollm2-360m-instruct-q8_0.p64",
        r"C:\LLM_Models\GGUF\smollm2-360m-instruct-q8_0.gguf",
        r"C:\LLM_Models\GGUF\lmstudio-community\smollm2-360m-instruct-q8_0.gguf",
    ];
    CANDIDATES
        .iter()
        .map(std::path::PathBuf::from)
        .find(|p| p.is_file())
}

/// Fixed factual probe used for coherence + throughput (same prompt as browser llmdemo gate).
pub const DECODE_PROXY_PROBE_PROMPT: &str = "The capital of France is";

/// Result of a decode-proxy measurement: speed **and** whether the completion is usable.
#[derive(Debug, Clone)]
pub struct DecodeProxyResult {
    pub tok_s: f64,
    pub text: String,
    /// True when completion contains the expected factual anchor (e.g. "Paris").
    pub coherence_ok: bool,
}

/// Greedy factual gate: completion must contain "paris" (case-insensitive).
/// Garbage token streams fail closed — speed alone is not excellence.
pub fn decode_proxy_coherence_ok(text: &str) -> bool {
    text.to_ascii_lowercase().contains("paris")
}

/// Run a short resident decode on `model` under the **current** process backend.
/// Returns tok/s + text + coherence, or `None` on hard failure.
///
/// Warm-up: one short decode builds pipelines / resident plan so the timed
/// measurement is not dominated by first-token cold cost.
///
/// Safe from CLI (which already owns a Tokio multi-thread runtime): nested
/// `block_on` panics, so we hop to a fresh OS thread for the measurement.
pub fn measure_decode_proxy(model: &Path, n_tokens: u32) -> Option<DecodeProxyResult> {
    crate::wgsl_forge::dispatch::ensure_cuda_runtime_path();
    let n = n_tokens.max(8).min(128);
    let path = model.to_path_buf();
    // Greedy, bounded — comparable across backends.
    crate::llm_bench::set_sampler_config(None);
    let path_str = path.to_str()?.to_string();
    let join = std::thread::Builder::new()
        .name("decode-proxy".into())
        .spawn(move || {
            let prompt = DECODE_PROXY_PROBE_PROMPT;
            // Warm: compile shaders + resident plan (discard rate).
            let _ = crate::llm_bench::decode_with_metrics_blocking(&path_str, prompt, 4);
            crate::llm_bench::decode_with_metrics_blocking(&path_str, prompt, n)
        })
        .ok()?
        .join();
    match join {
        Ok(Ok((text, tok_s))) if tok_s > 0.0 => {
            let coherence_ok = decode_proxy_coherence_ok(&text);
            Some(DecodeProxyResult {
                tok_s,
                text,
                coherence_ok,
            })
        }
        Ok(Ok(_)) => None,
        Ok(Err(e)) => {
            log::warn!("decode_proxy|fail|{e}");
            None
        }
        Err(_) => {
            log::warn!("decode_proxy|thread_panic");
            None
        }
    }
}

/// Back-compat: tok/s only (callers that ignore quality still work).
pub fn measure_decode_proxy_tok_s(model: &Path, n_tokens: u32) -> Option<f64> {
    measure_decode_proxy(model, n_tokens).map(|r| r.tok_s)
}

/// Attach decode-proxy tok/s to GPU circuits by spawning a child process per backend
/// (shared_gpu is process-wide OnceLock — cannot switch backends in-process).
///
/// `self_exe` should be the current CLI binary (`std::env::current_exe()`).
/// Child runs: `llm decode-proxy <model> --tokens N` with `QUALIA_WGPU_BACKEND` set.
pub fn attach_decode_proxy_via_subprocess(
    matrix: &mut CapabilityMatrix,
    model: &Path,
    n_tokens: u32,
    self_exe: &Path,
) {
    use std::process::Command;
    let n = n_tokens.max(8).min(64);
    // Measure only **discrete** GPU rows (iGPU would inherit a false tok/s if we
    // keyed only by backend token — wgpu picks the discrete card under QUALIA_WGPU_BACKEND).
    let mut seen_tokens = std::collections::HashSet::<String>::new();
    for c in matrix.circuits.iter_mut() {
        if c.kind != CircuitKind::DiscreteGpu {
            continue;
        }
        let Some(token) = backend_env_token(&c.backend) else {
            continue;
        };
        if !seen_tokens.insert(token.to_string()) {
            continue;
        }
        let output = Command::new(self_exe)
            .args([
                "llm",
                "decode-proxy",
                &model.display().to_string(),
                "--tokens",
                &n.to_string(),
            ])
            .env("QUALIA_WGPU_BACKEND", token)
            .env("QUALIA_P64_INTEGRITY", "metadata")
            .env("RUST_LOG", "error")
            .output();
        let tok_s = match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                parse_decode_proxy_line(&stdout)
            }
            Ok(o) => {
                log::warn!(
                    "decode_proxy|child_fail|backend={token}|{}",
                    String::from_utf8_lossy(&o.stderr)
                );
                None
            }
            Err(e) => {
                log::warn!("decode_proxy|spawn_fail|backend={token}|{e}");
                None
            }
        };
        c.decode_proxy_tok_s = tok_s;
        if let Some(t) = tok_s {
            log::info!("decode_proxy|ok|backend={token}|{t:.2} tok/s");
        }
    }
    // Copy to other discrete rows that share the same backend token only.
    let mut by_token: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for c in &matrix.circuits {
        if c.kind != CircuitKind::DiscreteGpu {
            continue;
        }
        if let (Some(tok), Some(t)) = (backend_env_token(&c.backend), c.decode_proxy_tok_s) {
            by_token.insert(tok.to_string(), t);
        }
    }
    for c in matrix.circuits.iter_mut() {
        if c.kind != CircuitKind::DiscreteGpu || c.decode_proxy_tok_s.is_some() {
            continue;
        }
        if let Some(tok) = backend_env_token(&c.backend) {
            if let Some(&t) = by_token.get(tok) {
                c.decode_proxy_tok_s = Some(t);
            }
        }
    }
    matrix.apply_decode_proxy_ranking();
}

/// Parse `DECODE_PROXY tok_s=12.34 backend=dx12 coherence=1` from child stdout.
pub fn parse_decode_proxy_line(stdout: &str) -> Option<f64> {
    parse_decode_proxy_record(stdout).map(|r| r.tok_s)
}

/// Full parse of the machine line from `llm decode-proxy`.
#[derive(Debug, Clone)]
pub struct DecodeProxyLine {
    pub tok_s: f64,
    pub coherence_ok: Option<bool>,
}

pub fn parse_decode_proxy_record(stdout: &str) -> Option<DecodeProxyLine> {
    for line in stdout.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("DECODE_PROXY ") {
            let mut tok_s: Option<f64> = None;
            let mut coherence_ok: Option<bool> = None;
            for part in rest.split_whitespace() {
                if let Some(v) = part.strip_prefix("tok_s=") {
                    tok_s = v.parse().ok();
                } else if let Some(v) = part.strip_prefix("coherence=") {
                    coherence_ok = Some(v == "1" || v.eq_ignore_ascii_case("true"));
                }
            }
            if let Some(tok_s) = tok_s {
                return Some(DecodeProxyLine {
                    tok_s,
                    coherence_ok,
                });
            }
        }
    }
    None
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
    // Prefer the stored token when present (stable across schema evolution).
    if let Some(ref t) = passport.preferred_inference_backend {
        return Some(t.clone());
    }
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
                    decode_proxy_tok_s: Some(18.0),
                },
                CircuitBench {
                    label: "CPU native".into(),
                    kind: CircuitKind::Cpu,
                    backend: "native".into(),
                    ms_per_gemv: 23.0,
                    gflops: 0.4,
                    upload_gbps: f64::INFINITY, // in-pool — must survive the round-trip
                    rel_score: 0.02,
                    decode_proxy_tok_s: None,
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
            preferred_inference_backend: Some("dx12".into()),
            probe_gemv_n: 2048,
            decode_proxy_model: None,
            decode_proxy_tokens: 0,
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

    #[test]
    fn parse_decode_proxy_line_extracts_tok_s() {
        let s = "noise\nDECODE_PROXY tok_s=12.50 backend=dx12 coherence=1\nmore\n";
        assert!((parse_decode_proxy_line(s).unwrap() - 12.5).abs() < 1e-9);
        let r = parse_decode_proxy_record(s).unwrap();
        assert_eq!(r.coherence_ok, Some(true));
        assert!(parse_decode_proxy_line("nope").is_none());
        assert!(decode_proxy_coherence_ok("… Paris is lovely"));
        assert!(!decode_proxy_coherence_ok("asdkfjhasdf"));
    }

    #[test]
    fn decode_proxy_ranking_prefers_higher_tok_s() {
        let mut matrix = CapabilityMatrix {
            circuits: vec![
                CircuitBench {
                    label: "fast gemv slow decode".into(),
                    kind: CircuitKind::DiscreteGpu,
                    backend: "Vulkan".into(),
                    ms_per_gemv: 0.1,
                    gflops: 50.0,
                    upload_gbps: 4.0,
                    rel_score: 1.0,
                    decode_proxy_tok_s: Some(5.0),
                },
                CircuitBench {
                    label: "slower gemv fast decode".into(),
                    kind: CircuitKind::DiscreteGpu,
                    backend: "Dx12".into(),
                    ms_per_gemv: 0.2,
                    gflops: 25.0,
                    upload_gbps: 4.0,
                    rel_score: 0.5,
                    decode_proxy_tok_s: Some(18.0),
                },
            ],
            gemv_n: 512,
            npu_probed: false,
        };
        matrix.apply_decode_proxy_ranking();
        assert_eq!(matrix.best().unwrap().backend, "Dx12");
        assert!((matrix.best().unwrap().rel_score - 1.0).abs() < 1e-9);
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
