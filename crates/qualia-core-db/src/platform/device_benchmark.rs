//! STELLAR §A AH-track H1(a) — **cross-circuit capability benchmark** (decisions D26/D30).
//!
//! "Rank by *measured* throughput, not a static device-type hierarchy" (Timothy). This runs an
//! identical representative GEMV on **every available compute circuit** — each wgpu adapter
//! (discrete GPU, integrated GPU) plus a native-Rust CPU path — and produces a **capability
//! matrix** sorted fastest-first. The residency/device planner (H2) consumes this to decide where
//! work goes; e.g. a weak iGPU can still beat PCIe-streaming for overflow, and a many-core CPU can
//! beat an old iGPU — only the numbers decide.
//!
//! Scope (honest): GPUs/iGPU via wgpu; CPU via a `rayon` GEMV. **NPU is not benchmarked** — NPU
//! access is a platform API (DirectML / NNAPI / CoreML), not wgpu, and is reported as "not probed".
//! This is part (a) of H1 (probe + matrix); the human-key *signing* of the passport (part (b)) is
//! blocked on the identity remediation (`identity-governance-remediation.md`) and lives elsewhere.
//!
//! Native-only (wgpu adapters when `gpu-runtime` is on, plus a `rayon` CPU GEMV row).
#![cfg(not(target_arch = "wasm32"))]
#![cfg_attr(not(feature = "gpu-runtime"), allow(dead_code))]

use serde::{Deserialize, Serialize};
#[cfg(feature = "gpu-runtime")]
use std::process::Command;

#[cfg(feature = "gpu-runtime")]
use etalon_diagnostics::device::backend_name;
use etalon_diagnostics::ipc::{decode_response, encode_response};
use etalon_diagnostics::measurement::{bench_cpu_gemv, gflops};
pub use etalon_diagnostics::{
    CircuitBench, CircuitKind, DeviceBenchmarkRequest, DeviceBenchmarkResponse,
};

#[cfg(feature = "gpu-runtime")]
fn backend_rank(b: wgpu::Backend) -> u8 {
    match b {
        wgpu::Backend::Metal => 0,
        wgpu::Backend::Dx12 => 1,
        wgpu::Backend::Vulkan => 2,
        wgpu::Backend::Gl => 3,
        _ => 4,
    }
}

#[cfg(not(test))]
const WORKER_ENV: &str = "QUALIA_DEVICE_BENCHMARK_WORKER";
const WORKER_OUTPUT_ENV: &str = "QUALIA_DEVICE_BENCHMARK_OUTPUT";

/// The measured capability matrix — circuits sorted fastest-first. This IS the priority order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMatrix {
    pub circuits: Vec<CircuitBench>,
    pub gemv_n: usize,
    /// NPU left unprobed (no portable compute path); recorded for honesty.
    pub npu_probed: bool,
}

impl CapabilityMatrix {
    /// Highest-throughput circuit, if any.
    pub fn best(&self) -> Option<&CircuitBench> {
        self.circuits.first()
    }

    /// Summary.
    pub fn summary(&self) -> String {
        let mut s = format!(
            "CapabilityMatrix (GEMV {n}x{n}, ranked by measured throughput; NPU probed={npu}):\n",
            n = self.gemv_n,
            npu = self.npu_probed
        );
        for (i, c) in self.circuits.iter().enumerate() {
            let upload = if c.upload_gbps.is_infinite() {
                "in-pool".to_string()
            } else {
                format!("{:.1} GB/s up", c.upload_gbps)
            };
            let decode = match c.decode_proxy_tok_s {
                Some(t) => format!("{t:.2} tok/s"),
                None => "—".into(),
            };
            s.push_str(&format!(
                "  {}. {:<28} [{:?}/{}] {:>8.3} ms  {:>7.1} GFLOP/s  {:>12}  decode {:>10}  score {:.3}  timing {:?}\n",
                i + 1,
                c.label,
                c.kind,
                c.backend,
                c.ms_per_gemv,
                c.gflops,
                upload,
                decode,
                c.rel_score,
                c.timing_source(),
            ));
        }
        s
    }

    /// Re-rank circuits: prefer higher `decode_proxy_tok_s` when present on any GPU row;
    /// otherwise keep GEMV ranking. CPU rows without decode stay at the bottom of GPU ranking.
    pub fn apply_decode_proxy_ranking(&mut self) {
        let any_decode = self
            .circuits
            .iter()
            .any(|c| c.decode_proxy_tok_s.is_some() && c.kind != CircuitKind::Cpu);
        if !any_decode {
            return;
        }
        self.circuits.sort_by(|a, b| {
            let ta = a.decode_proxy_tok_s.unwrap_or(-1.0);
            let tb = b.decode_proxy_tok_s.unwrap_or(-1.0);
            // Higher tok/s first; unmeasured (-1) after measured.
            match tb.partial_cmp(&ta).unwrap_or(std::cmp::Ordering::Equal) {
                std::cmp::Ordering::Equal => a
                    .ms_per_gemv
                    .partial_cmp(&b.ms_per_gemv)
                    .unwrap_or(std::cmp::Ordering::Equal),
                o => o,
            }
        });
        if let Some(best_t) = self
            .circuits
            .iter()
            .filter_map(|c| c.decode_proxy_tok_s)
            .fold(None, |acc: Option<f64>, t| {
                Some(acc.map(|a| a.max(t)).unwrap_or(t))
            })
        {
            for c in &mut self.circuits {
                c.rel_score = match c.decode_proxy_tok_s {
                    Some(t) if best_t > 0.0 => t / best_t,
                    _ => 0.0,
                };
            }
        }
    }
}

/// Worker entry used by the dedicated binary and the unit-test subprocess route.
#[cfg(feature = "gpu-runtime")]
pub fn run_worker_from_env() -> Result<(), String> {
    let request: DeviceBenchmarkRequest = DeviceBenchmarkRequest {
        backend: std::env::var("QUALIA_DEVICE_BENCHMARK_BACKEND").map_err(|_| "missing backend")?,
        vendor: std::env::var("QUALIA_DEVICE_BENCHMARK_VENDOR")
            .map_err(|_| "missing vendor")?
            .parse()
            .map_err(|_| "invalid vendor")?,
        device: std::env::var("QUALIA_DEVICE_BENCHMARK_DEVICE")
            .map_err(|_| "missing device")?
            .parse()
            .map_err(|_| "invalid device")?,
        gemv_n: std::env::var("QUALIA_DEVICE_BENCHMARK_N")
            .map_err(|_| "missing n")?
            .parse()
            .map_err(|_| "invalid n")?,
    };
    let response = match etalon_diagnostics::device::benchmark_one(&request) {
        Ok(bench) => DeviceBenchmarkResponse {
            request,
            bench: Some(bench),
            error: None,
        },
        Err(error) => DeviceBenchmarkResponse {
            request,
            bench: None,
            error: Some(error),
        },
    };
    let output = std::env::var_os(WORKER_OUTPUT_ENV).ok_or("missing worker output path")?;
    encode_response(std::path::Path::new(&output), &response)
}

#[cfg(not(test))]
fn worker_executable() -> Option<std::path::PathBuf> {
    if let Some(path) = std::env::var_os(WORKER_ENV) {
        return Some(path.into());
    }
    let current = std::env::current_exe().ok()?;
    let sibling = current.with_file_name(if cfg!(windows) {
        "qualia-device-benchmark-worker.exe"
    } else {
        "qualia-device-benchmark-worker"
    });
    if sibling.is_file() {
        Some(sibling)
    } else if current
        .file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(|name| name == "qualia-cli" || name == "webizen-desktop")
    {
        // Qualia's shipped CLI and desktop hosts expose the same private worker
        // entry before normal argument/UI initialization, so no sidecar is
        // required for those packages. Other embedders can set WORKER_ENV.
        Some(current)
    } else {
        None
    }
}

#[cfg(feature = "gpu-runtime")]
fn invoke_worker(
    request: &DeviceBenchmarkRequest,
    cancelled: &dyn Fn() -> bool,
) -> Result<CircuitBench, String> {
    let artifacts = etalon_diagnostics::ipc::WorkerArtifacts::new()?;
    let output = artifacts.response_path();
    let mut command;
    #[cfg(test)]
    {
        command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
        command.args([
            "--exact",
            "platform::device_benchmark::tests::device_benchmark_worker_entry",
            "--nocapture",
        ]);
    }
    #[cfg(not(test))]
    {
        command = Command::new(
            worker_executable().ok_or_else(|| format!("worker not found; set {WORKER_ENV}"))?,
        );
    }
    command
        .env("QUALIA_DEVICE_BENCHMARK_BACKEND", &request.backend)
        .env("QUALIA_DEVICE_BENCHMARK_VENDOR", request.vendor.to_string())
        .env("QUALIA_DEVICE_BENCHMARK_DEVICE", request.device.to_string())
        .env("QUALIA_DEVICE_BENCHMARK_N", request.gemv_n.to_string())
        .env(WORKER_OUTPUT_ENV, &output);
    etalon_diagnostics::process::run_worker(
        &mut command,
        &etalon_diagnostics::process::WorkerLimits::default(),
        cancelled,
        &|| artifacts.check_budget(),
    )?;
    etalon_diagnostics::ipc::decode_matching_response(&output, request)
}

/// Benchmark every available compute circuit and return the ranked capability matrix.
///
/// `n` is the GEMV side length (representative shape; 2048 is a good fast default).
/// **Each (vendor, device, backend) triple is benchmarked separately** so DX12 vs Vulkan
/// (same physical GPU) can rank against each other — the whole point of the passport.
/// The software/WARP "CPU" wgpu adapter is skipped — the native `rayon` path is the honest CPU number.
pub fn benchmark_devices(n: usize) -> CapabilityMatrix {
    match benchmark_devices_cancellable(n, &|| false) {
        Ok(matrix) => matrix,
        Err(error) => {
            log::warn!("device_benchmark|failed|{error}");
            CapabilityMatrix {
                circuits: Vec::new(),
                gemv_n: n,
                npu_probed: false,
            }
        }
    }
}

/// Cancellation aborts the matrix and kills the currently isolated adapter worker.
pub fn benchmark_devices_cancellable(
    n: usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<CapabilityMatrix, String> {
    let mut circuits: Vec<CircuitBench> = Vec::new();
    if cancelled() {
        return Err("device benchmark cancelled".into());
    }
    etalon_diagnostics::measurement::validate_shape(
        n,
        etalon_diagnostics::measurement::DEFAULT_BENCH_BYTES,
    )?;

    // ── GPUs / iGPU via wgpu — one circuit row per backend that can open the device ──
    #[cfg(feature = "gpu-runtime")]
    {
        let instance = wgpu::Instance::default();
        let mut cand: Vec<(u8, wgpu::Adapter, wgpu::AdapterInfo)> = Vec::new();
        for adapter in pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all())) {
            let info = adapter.get_info();
            if info.device_type == wgpu::DeviceType::Cpu || info.device == 0 {
                continue;
            }
            cand.push((backend_rank(info.backend), adapter, info));
        }
        cand.sort_by_key(|(r, _, _)| *r);
        // Dedup exact (vendor, device, backend) only — keep Metal+DX12+Vulkan rows for the same card.
        let mut seen: std::collections::HashSet<(u32, u32, u32)> = std::collections::HashSet::new();
        let mut chosen: Vec<DeviceBenchmarkRequest> = Vec::new();
        for (_, _adapter, info) in cand {
            let backend_id = info.backend as u32;
            if seen.insert((info.vendor, info.device, backend_id)) {
                chosen.push(DeviceBenchmarkRequest {
                    backend: backend_name(info.backend).to_string(),
                    vendor: info.vendor,
                    device: info.device,
                    gemv_n: n,
                });
            }
        }
        for request in chosen {
            if cancelled() {
                break;
            }
            match invoke_worker(&request, cancelled) {
                Ok(bench) => circuits.push(bench),
                Err(error) => log::warn!(
                    "device_benchmark|skip|{:04x}:{:04x}|{}|{}",
                    request.vendor,
                    request.device,
                    request.backend,
                    error
                ),
            }
        }
    }

    if cancelled() {
        return Err("device benchmark cancelled".into());
    }
    // ── CPU via native rayon ──
    let cpu_ms = bench_cpu_gemv(n, etalon_diagnostics::measurement::DEFAULT_BENCH_BYTES)?;
    if cancelled() {
        return Err("device benchmark cancelled".into());
    }
    circuits.push(CircuitBench {
        label: format!("CPU native (rayon, {} cores)", num_cpus::get()),
        kind: CircuitKind::Cpu,
        backend: "native".to_string(),
        ms_per_gemv: cpu_ms,
        gflops: gflops(n, cpu_ms),
        upload_gbps: f64::INFINITY, // data already in the CPU's pool — no transfer
        rel_score: 1.0,
        decode_proxy_tok_s: None,
    });

    // Rank fastest-first and fill relative scores.
    circuits.sort_by(|a, b| {
        a.ms_per_gemv
            .partial_cmp(&b.ms_per_gemv)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.backend.cmp(&b.backend))
            .then_with(|| a.label.cmp(&b.label))
    });
    if let Some(best_ms) = circuits.first().map(|c| c.ms_per_gemv) {
        for c in &mut circuits {
            c.rel_score = if c.ms_per_gemv > 0.0 {
                best_ms / c.ms_per_gemv
            } else {
                0.0
            };
        }
    }

    Ok(CapabilityMatrix {
        circuits,
        gemv_n: n,
        npu_probed: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "gpu-runtime")]
    fn device_benchmark_worker_entry() {
        if std::env::var_os(WORKER_OUTPUT_ENV).is_some() {
            run_worker_from_env().expect("device benchmark worker must write its response");
        }
    }

    #[test]
    fn worker_protocol_round_trips() {
        let request = DeviceBenchmarkRequest {
            backend: "vulkan".into(),
            vendor: 1,
            device: 2,
            gemv_n: 32,
        };
        let response = DeviceBenchmarkResponse {
            request: request.clone(),
            bench: None,
            error: Some("expected".into()),
        };
        let artifacts = etalon_diagnostics::ipc::WorkerArtifacts::new().unwrap();
        let path = artifacts.response_path();
        encode_response(&path, &response).unwrap();
        let decoded = decode_response(&path).unwrap();
        assert_eq!(decoded.request.backend, request.backend);
        assert_eq!(decoded.error.as_deref(), Some("expected"));
    }

    /// Cross-circuit benchmark on whatever silicon is present. Prints the ranked matrix; asserts the
    /// CPU path always appears and the ranking is consistent. Skips GPU rows cleanly if headless.
    #[test]
    #[serial_test::serial(gpu)]
    fn h1a_capability_matrix() {
        let matrix = benchmark_devices(2048);
        eprintln!("{}", matrix.summary());

        assert!(
            !matrix.circuits.is_empty(),
            "at least the CPU circuit must be benchmarked"
        );
        assert!(
            matrix.circuits.iter().any(|c| c.kind == CircuitKind::Cpu),
            "native CPU circuit must always be present"
        );
        // Sorted fastest-first → non-decreasing ms, non-increasing rel_score.
        for w in matrix.circuits.windows(2) {
            assert!(
                w[0].ms_per_gemv <= w[1].ms_per_gemv + 1e-9,
                "matrix must be sorted fastest-first"
            );
        }
        assert!(
            (matrix.best().unwrap().rel_score - 1.0).abs() < 1e-9,
            "best score must be 1.0"
        );
    }
}
