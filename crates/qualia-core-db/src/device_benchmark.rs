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

use serde::Serialize;
use std::time::Instant;

const GEMV_BENCH_WGSL: &str = include_str!("shaders/gemv_bench.wgsl");

/// A compute circuit's class in the capability matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
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
    /// Relative score in [0,1]: fastest circuit = 1.0, others = fastest_ms / this_ms.
    pub rel_score: f64,
}

/// The measured capability matrix — circuits sorted fastest-first. This IS the priority order.
#[derive(Debug, Clone, Serialize)]
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
            s.push_str(&format!(
                "  {}. {:<28} [{:?}/{}] {:>8.3} ms  {:>7.1} GFLOP/s  {:>12}  score {:.3}\n",
                i + 1,
                c.label,
                c.kind,
                c.backend,
                c.ms_per_gemv,
                c.gflops,
                upload,
                c.rel_score,
            ));
        }
        s
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
        entry_point: "gemv",
        compilation_options: Default::default(),
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
            wgpu::BindGroupEntry { binding: 0, resource: input.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: weight.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: params.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: out.as_entire_binding() },
        ],
    });
    let wg_x = (n as u32).div_ceil(64).max(1);
    let (k, s) = (16u32, 5u32);
    let submit_batch = || {
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
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
        let _ = device.poll(wgpu::Maintain::Wait);
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
        let _ = device.poll(wgpu::Maintain::Wait);
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

/// Benchmark every available compute circuit and return the ranked capability matrix.
///
/// `n` is the GEMV side length (representative shape; 2048 is a good fast default). Each physical
/// GPU is benchmarked once (deduped across backends). The software/WARP "CPU" wgpu adapter is
/// skipped — the native `rayon` path is the honest CPU number.
pub fn benchmark_devices(n: usize) -> CapabilityMatrix {
    let mut circuits: Vec<CircuitBench> = Vec::new();

    // ── GPUs / iGPU via wgpu (dedup physical device across backends) ──
    // `wgpu::Adapter` is not `Clone`, so dedup by sorting candidates by backend rank then keeping
    // the first (best) occurrence per (vendor, device). Skip software CPU adapters + GL phantoms.
    let instance = wgpu::Instance::default();
    let mut cand: Vec<(u8, wgpu::Adapter, wgpu::AdapterInfo)> = Vec::new();
    for adapter in instance.enumerate_adapters(wgpu::Backends::all()) {
        let info = adapter.get_info();
        if info.device_type == wgpu::DeviceType::Cpu || info.device == 0 {
            continue;
        }
        cand.push((backend_rank(info.backend), adapter, info));
    }
    cand.sort_by_key(|(r, _, _)| *r);
    let mut seen: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
    let mut chosen: Vec<(wgpu::Adapter, wgpu::AdapterInfo)> = Vec::new();
    for (_, adapter, info) in cand {
        if seen.insert((info.vendor, info.device)) {
            chosen.push((adapter, info));
        }
    }
    for (adapter, info) in chosen {
        let Some((device, queue)) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None)).ok()
        else {
            log::warn!("device_benchmark|skip|{}|request_device failed", info.name);
            continue;
        };
        let ms = bench_gpu_gemv(&device, &queue, n);
        let upload_gbps = bench_upload_gbps(&device, &queue, 128 * 1024 * 1024);
        circuits.push(CircuitBench {
            label: info.name.clone(),
            kind: CircuitKind::from_wgpu(info.device_type),
            backend: format!("{:?}", info.backend),
            ms_per_gemv: ms,
            gflops: gflops(n, ms),
            upload_gbps,
            rel_score: 1.0, // filled after sort
        });
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
    });

    // Rank fastest-first and fill relative scores.
    circuits.sort_by(|a, b| a.ms_per_gemv.partial_cmp(&b.ms_per_gemv).unwrap_or(std::cmp::Ordering::Equal));
    if let Some(best_ms) = circuits.first().map(|c| c.ms_per_gemv) {
        for c in &mut circuits {
            c.rel_score = if c.ms_per_gemv > 0.0 { best_ms / c.ms_per_gemv } else { 0.0 };
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

    /// Cross-circuit benchmark on whatever silicon is present. Prints the ranked matrix; asserts the
    /// CPU path always appears and the ranking is consistent. Skips GPU rows cleanly if headless.
    #[test]
    fn h1a_capability_matrix() {
        let matrix = benchmark_devices(2048);
        eprintln!("{}", matrix.summary());

        assert!(!matrix.circuits.is_empty(), "at least the CPU circuit must be benchmarked");
        assert!(
            matrix.circuits.iter().any(|c| c.kind == CircuitKind::Cpu),
            "native CPU circuit must always be present"
        );
        // Sorted fastest-first → non-decreasing ms, non-increasing rel_score.
        for w in matrix.circuits.windows(2) {
            assert!(w[0].ms_per_gemv <= w[1].ms_per_gemv + 1e-9, "matrix must be sorted fastest-first");
        }
        assert!((matrix.best().unwrap().rel_score - 1.0).abs() < 1e-9, "best score must be 1.0");
    }
}
