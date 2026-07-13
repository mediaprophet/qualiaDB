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
//! Native only.
#![cfg(not(target_arch = "wasm32"))]

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::time::Instant;

const GEMV_BENCH_WGSL: &str = include_str!("../shaders/gemv_bench.wgsl");

/// A compute circuit's class in the capability matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitKind {
    DiscreteGpu,
    IntegratedGpu,
    Cpu,
    Npu,
    Other,
}

impl CircuitKind {
    fn from_wgpu(t: wgpu::DeviceType) -> Self {
        match t {
            wgpu::DeviceType::DiscreteGpu => Self::DiscreteGpu,
            wgpu::DeviceType::IntegratedGpu => Self::IntegratedGpu,
            wgpu::DeviceType::Cpu => Self::Cpu,
            _ => Self::Other,
        }
    }
}

/// One benchmarked circuit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBench {
    pub label: String,
    pub kind: CircuitKind,
    pub backend: String,
    /// Milliseconds per GEMV dispatch (lower is faster).
    pub ms_per_gemv: f64,
    /// Effective GFLOP/s for the GEMV (2·n·n flops / time).
    pub gflops: f64,
    /// Host→device upload bandwidth (GB/s) for a representative buffer. The **transfer axis** (D31):
    /// PCIe for a discrete GPU; staging-path for an iGPU (wgpu can't show true zero-copy → relative
    /// signal); `f64::INFINITY` for the CPU (data is already in its pool — no transfer). Decode that
    /// streams weights to a device pays this every token; in-pool compute does not.
    pub upload_gbps: f64,
    /// Relative score in [0,1]: fastest circuit = 1.0, others = fastest_ms / this_ms
    /// (or highest decode_proxy_tok_s when decode ranking is active).
    pub rel_score: f64,
    /// Optional real-decode proxy (tok/s) from a short resident decode on a small model.
    /// When present for ≥1 GPU circuit, passport ranking prefers this over GEMV µs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decode_proxy_tok_s: Option<f64>,
}

/// Stable process boundary for one physical adapter/backend benchmark.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceBenchmarkRequest {
    pub backend: String,
    pub vendor: u32,
    pub device: u32,
    pub gemv_n: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceBenchmarkResponse {
    pub request: DeviceBenchmarkRequest,
    pub bench: Option<CircuitBench>,
    pub error: Option<String>,
}

#[cfg(not(test))]
const WORKER_ENV: &str = "QUALIA_DEVICE_BENCHMARK_WORKER";
const WORKER_OUTPUT_ENV: &str = "QUALIA_DEVICE_BENCHMARK_OUTPUT";
static WORKER_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
                "  {}. {:<28} [{:?}/{}] {:>8.3} ms  {:>7.1} GFLOP/s  {:>12}  decode {:>10}  score {:.3}\n",
                i + 1,
                c.label,
                c.kind,
                c.backend,
                c.ms_per_gemv,
                c.gflops,
                upload,
                decode,
                c.rel_score,
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

#[inline]
fn params_bytes(n_in: u32, n_out: u32) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[0..4].copy_from_slice(&n_in.to_le_bytes());
    b[4..8].copy_from_slice(&n_out.to_le_bytes());
    b
}

/// Persistent-pipeline GEMV timing on one wgpu device (ms per dispatch). No readback — we poll to
/// completion so the timing reflects execution, with submit overhead amortized over K dispatches.
fn bench_gpu_gemv(device: &wgpu::Device, queue: &wgpu::Queue, n: usize) -> f64 {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gemv_bench"),
        source: wgpu::ShaderSource::Wgsl(GEMV_BENCH_WGSL.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("gemv_bench_pipeline"),
        layout: None,
        module: &shader,
        entry_point: Some("gemv"),
        compilation_options: Default::default(),
        cache: None,
    });
    let mk = |contents: &[u8], usage: wgpu::BufferUsages| {
        let b = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: contents.len().max(4) as u64,
            usage,
            mapped_at_creation: false,
        });
        if !contents.is_empty() {
            queue.write_buffer(&b, 0, contents);
        }
        b
    };
    let input = mk(
        bytemuck::cast_slice(&vec![0.1f32; n]),
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    );
    let weight = mk(
        bytemuck::cast_slice(&vec![0.05f32; n * n]),
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    );
    let params = mk(
        &params_bytes(n as u32, n as u32),
        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    );
    let out = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (n * 4).max(4) as u64,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: weight.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: out.as_entire_binding(),
            },
        ],
    });
    let wg_x = (n as u32).div_ceil(64).max(1);
    let (k, s) = (16u32, 5u32);
    let submit_batch = || {
        let mut enc =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind, &[]);
            for _ in 0..k {
                pass.dispatch_workgroups(wg_x, 1, 1);
            }
        }
        queue.submit(Some(enc.finish()));
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
    };
    submit_batch(); // warmup
    let t0 = Instant::now();
    for _ in 0..s {
        submit_batch();
    }
    t0.elapsed().as_secs_f64() * 1e3 / (k * s) as f64
}

/// Host→device upload bandwidth (GB/s) for a `bytes`-sized buffer — the transfer axis (D31).
/// Times `write_buffer` + a flushing submit + `poll(Wait)` so the upload is realized. For a discrete
/// GPU this is the PCIe cost; for an iGPU it's the wgpu staging path (not the true near-zero of a
/// unified pool — a relative signal, flagged honestly).
fn bench_upload_gbps(device: &wgpu::Device, queue: &wgpu::Queue, bytes: usize) -> f64 {
    let data = vec![0u8; bytes];
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("upload_probe"),
        size: bytes as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let flush = || {
        let enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        queue.submit(Some(enc.finish()));
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
    };
    queue.write_buffer(&buf, 0, &data); // warmup
    flush();
    let iters = 3u32;
    let t0 = Instant::now();
    for _ in 0..iters {
        queue.write_buffer(&buf, 0, &data);
        flush();
    }
    let secs = t0.elapsed().as_secs_f64() / iters as f64;
    if secs <= 0.0 {
        0.0
    } else {
        bytes as f64 / secs / 1e9
    }
}

/// Native-Rust (`rayon`) GEMV timing (ms per GEMV) — the CPU compute path, fairly multi-threaded.
fn bench_cpu_gemv(n: usize) -> f64 {
    use rayon::prelude::*;
    let weight = vec![0.05f32; n * n];
    let input = vec![0.1f32; n];
    let mut out = vec![0.0f32; n];
    let gemv = |out: &mut [f32]| {
        out.par_iter_mut().enumerate().for_each(|(i, o)| {
            let row = &weight[i * n..(i + 1) * n];
            *o = row.iter().zip(input.iter()).map(|(a, b)| a * b).sum();
        });
    };
    gemv(&mut out); // warmup
    let iters = 5u32;
    let t0 = Instant::now();
    for _ in 0..iters {
        gemv(&mut out);
    }
    t0.elapsed().as_secs_f64() * 1e3 / iters as f64
}

#[inline]
fn gflops(n: usize, ms: f64) -> f64 {
    if ms <= 0.0 {
        0.0
    } else {
        (2.0 * n as f64 * n as f64) / (ms / 1e3) / 1e9
    }
}

fn backend_rank(b: wgpu::Backend) -> u8 {
    match b {
        wgpu::Backend::Metal => 0,
        wgpu::Backend::Dx12 => 1,
        wgpu::Backend::Vulkan => 2,
        wgpu::Backend::Gl => 3,
        _ => 4,
    }
}

fn backend_name(backend: wgpu::Backend) -> &'static str {
    match backend {
        wgpu::Backend::Vulkan => "vulkan",
        wgpu::Backend::Dx12 => "dx12",
        wgpu::Backend::Metal => "metal",
        wgpu::Backend::Gl => "gl",
        wgpu::Backend::BrowserWebGpu => "browser-webgpu",
        wgpu::Backend::Noop => "noop",
    }
}

fn backend_from_name(name: &str) -> Option<(wgpu::Backend, wgpu::Backends)> {
    match name {
        "vulkan" => Some((wgpu::Backend::Vulkan, wgpu::Backends::VULKAN)),
        "dx12" => Some((wgpu::Backend::Dx12, wgpu::Backends::DX12)),
        "metal" => Some((wgpu::Backend::Metal, wgpu::Backends::METAL)),
        "gl" => Some((wgpu::Backend::Gl, wgpu::Backends::GL)),
        _ => None,
    }
}

fn encode_response(path: &std::path::Path, response: &DeviceBenchmarkResponse) -> Result<(), String> {
    let mut payload = Vec::new();
    ciborium::into_writer(response, &mut payload).map_err(|e| format!("encode response: {e}"))?;
    let mut file = std::fs::File::create(path).map_err(|e| format!("create {}: {e}", path.display()))?;
    file.write_all(&(payload.len() as u64).to_le_bytes()).map_err(|e| e.to_string())?;
    file.write_all(&payload).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}

fn decode_response(path: &std::path::Path) -> Result<DeviceBenchmarkResponse, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    let mut length = [0u8; 8];
    file.read_exact(&mut length).map_err(|e| format!("read response length: {e}"))?;
    let length = u64::from_le_bytes(length) as usize;
    if length == 0 || length > 1024 * 1024 {
        return Err(format!("invalid worker response length {length}"));
    }
    let mut payload = vec![0u8; length];
    file.read_exact(&mut payload).map_err(|e| format!("read response: {e}"))?;
    ciborium::from_reader(payload.as_slice()).map_err(|e| format!("decode response: {e}"))
}

fn benchmark_one(request: &DeviceBenchmarkRequest) -> Result<CircuitBench, String> {
    let (expected_backend, backends) = backend_from_name(&request.backend)
        .ok_or_else(|| format!("unsupported backend {}", request.backend))?;
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = backends;
    let instance = wgpu::Instance::new(descriptor);
    let adapters = pollster::block_on(instance.enumerate_adapters(backends));
    let adapter = adapters.into_iter().find(|adapter| {
        let info = adapter.get_info();
        info.backend == expected_backend && info.vendor == request.vendor && info.device == request.device
    }).ok_or_else(|| format!("adapter {:04x}:{:04x}/{} unavailable", request.vendor, request.device, request.backend))?;
    let info = adapter.get_info();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        .map_err(|e| format!("request_device: {e}"))?;
    let ms = bench_gpu_gemv(&device, &queue, request.gemv_n);
    let upload_gbps = bench_upload_gbps(&device, &queue, 16 * 1024 * 1024);
    Ok(CircuitBench {
        label: format!("{} ({:?})", info.name, info.backend),
        kind: CircuitKind::from_wgpu(info.device_type),
        backend: format!("{:?}", info.backend),
        ms_per_gemv: ms,
        gflops: gflops(request.gemv_n, ms),
        upload_gbps,
        rel_score: 1.0,
        decode_proxy_tok_s: None,
    })
}

/// Worker entry used by the dedicated binary and the unit-test subprocess route.
pub fn run_worker_from_env() -> Result<(), String> {
    let request: DeviceBenchmarkRequest = DeviceBenchmarkRequest {
        backend: std::env::var("QUALIA_DEVICE_BENCHMARK_BACKEND").map_err(|_| "missing backend")?,
        vendor: std::env::var("QUALIA_DEVICE_BENCHMARK_VENDOR").map_err(|_| "missing vendor")?.parse().map_err(|_| "invalid vendor")?,
        device: std::env::var("QUALIA_DEVICE_BENCHMARK_DEVICE").map_err(|_| "missing device")?.parse().map_err(|_| "invalid device")?,
        gemv_n: std::env::var("QUALIA_DEVICE_BENCHMARK_N").map_err(|_| "missing n")?.parse().map_err(|_| "invalid n")?,
    };
    let response = match benchmark_one(&request) {
        Ok(bench) => DeviceBenchmarkResponse { request, bench: Some(bench), error: None },
        Err(error) => DeviceBenchmarkResponse { request, bench: None, error: Some(error) },
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
    } else if current.file_stem().and_then(|s| s.to_str()).is_some_and(|name| {
        name == "qualia-cli" || name == "webizen-desktop"
    }) {
        // Qualia's shipped CLI and desktop hosts expose the same private worker
        // entry before normal argument/UI initialization, so no sidecar is
        // required for those packages. Other embedders can set WORKER_ENV.
        Some(current)
    } else {
        None
    }
}

fn invoke_worker(request: &DeviceBenchmarkRequest) -> Result<CircuitBench, String> {
    let sequence = WORKER_SEQUENCE.fetch_add(1, AtomicOrdering::Relaxed);
    let output = std::env::temp_dir().join(format!("qualia-device-bench-{}-{sequence}.cbor", std::process::id()));
    let mut command;
    #[cfg(test)]
    {
        command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
        command.args(["--exact", "platform::device_benchmark::tests::device_benchmark_worker_entry", "--nocapture"]);
    }
    #[cfg(not(test))]
    {
        command = Command::new(worker_executable().ok_or_else(|| format!("worker not found; set {WORKER_ENV}"))?);
    }
    let mut child = command
        .env("QUALIA_DEVICE_BENCHMARK_BACKEND", &request.backend)
        .env("QUALIA_DEVICE_BENCHMARK_VENDOR", request.vendor.to_string())
        .env("QUALIA_DEVICE_BENCHMARK_DEVICE", request.device.to_string())
        .env("QUALIA_DEVICE_BENCHMARK_N", request.gemv_n.to_string())
        .env(WORKER_OUTPUT_ENV, &output)
        .spawn().map_err(|e| format!("launch worker: {e}"))?;
    let deadline = Instant::now() + std::time::Duration::from_secs(120);
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| format!("wait for worker: {e}"))? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&output);
            return Err("worker exceeded 120-second deadline".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    };
    let decoded = if status.success() { decode_response(&output) } else { Err(format!("worker exited {status}")) };
    let _ = std::fs::remove_file(&output);
    let response = decoded?;
    if response.request.backend != request.backend || response.request.vendor != request.vendor
        || response.request.device != request.device || response.request.gemv_n != request.gemv_n {
        return Err("worker response identity mismatch".into());
    }
    if let Some(error) = response.error { return Err(error); }
    let bench = response.bench.ok_or("worker returned neither result nor error")?;
    if !bench.ms_per_gemv.is_finite() || bench.ms_per_gemv <= 0.0 || !bench.gflops.is_finite()
        || !bench.upload_gbps.is_finite() || bench.upload_gbps < 0.0 {
        return Err("worker returned invalid metrics".into());
    }
    Ok(bench)
}

/// Benchmark every available compute circuit and return the ranked capability matrix.
///
/// `n` is the GEMV side length (representative shape; 2048 is a good fast default).
/// **Each (vendor, device, backend) triple is benchmarked separately** so DX12 vs Vulkan
/// (same physical GPU) can rank against each other — the whole point of the passport.
/// The software/WARP "CPU" wgpu adapter is skipped — the native `rayon` path is the honest CPU number.
pub fn benchmark_devices(n: usize) -> CapabilityMatrix {
    let mut circuits: Vec<CircuitBench> = Vec::new();

    // ── GPUs / iGPU via wgpu — one circuit row per backend that can open the device ──
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
        match invoke_worker(&request) {
            Ok(bench) => circuits.push(bench),
            Err(error) => log::warn!(
                "device_benchmark|skip|{:04x}:{:04x}|{}|{}",
                request.vendor, request.device, request.backend, error
            ),
        }
        // Guard: if a backend hangs the probe, the process may stick — operators can
        // Each worker has a hard deadline, so a wedged backend is skipped without
        // poisoning the parent or preventing the remaining adapters from running.
    }

    // ── CPU via native rayon ──
    let cpu_ms = bench_cpu_gemv(n);
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

    CapabilityMatrix {
        circuits,
        gemv_n: n,
        npu_probed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
        let path = std::env::temp_dir().join(format!(
            "qualia-device-protocol-{}.cbor",
            std::process::id()
        ));
        encode_response(&path, &response).unwrap();
        let decoded = decode_response(&path).unwrap();
        let _ = std::fs::remove_file(path);
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
