//! The portable **wgpu f32 GEMM** — the execute half of the bridge. Until now
//! `compute_bridge` only *probed* and *planned*; this actually runs a kernel on the GPU
//! and reads the result back, reusing the exact wgpu idioms the capability benchmark
//! ([`crate::device_benchmark`]) already uses on this hardware.
//!
//! It is **f32** by design: portable GPUs (and WGSL) are f32, and this is the precision
//! the throughput-bound consumers (KGE scoring, dense swarm jobs, and later the GGUF
//! inference path) actually want. The f64 scientific kernels stay on the CPU reference —
//! the bridge never silently downcasts them.
//!
//! Correctness is gated against the CPU reference before this path may be a default
//! (§13); see the test and [`super::execute`].

use std::sync::mpsc;
use std::sync::OnceLock;

/// A ready-to-dispatch GEMM context: one device/queue/pipeline, built once.
pub struct WgpuGemm {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    /// `maxStorageBufferBindingSize` — a job whose largest buffer exceeds this falls
    /// back to CPU (no tiling yet; a correct fallback, not a stub).
    max_buffer_bytes: u64,
    pub adapter_label: String,
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

impl WgpuGemm {
    /// Build the context on the best available non-CPU adapter, or `None` if headless.
    fn create() -> Option<Self> {
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
        let (_, adapter, info) = cand.into_iter().next()?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
        let max_buffer_bytes = device.limits().max_storage_buffer_binding_size as u64;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gemm_substrate"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../shaders/gemm_substrate.wgsl").into(),
            ),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("gemm_substrate_pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some("gemm"),
            compilation_options: Default::default(),
            cache: None,
        });
        Some(Self {
            device,
            queue,
            pipeline,
            max_buffer_bytes,
            adapter_label: format!("{} [{:?}]", info.name, info.backend),
        })
    }

    /// Can this context run an `m×k · k×n` job without exceeding the buffer limit?
    pub fn fits(&self, m: usize, k: usize, n: usize) -> bool {
        let f32_bytes = |elems: usize| (elems * 4) as u64;
        f32_bytes(m * k) <= self.max_buffer_bytes
            && f32_bytes(k * n) <= self.max_buffer_bytes
            && f32_bytes(m * n) <= self.max_buffer_bytes
    }

    /// `C = A · B` on the GPU. `a` is `m×k`, `b` is `k×n`, result is `m×n` row-major.
    /// Returns `None` if the job does not fit (caller falls back to CPU).
    pub fn gemm(&self, m: usize, k: usize, n: usize, a: &[f32], b: &[f32]) -> Option<Vec<f32>> {
        if a.len() != m * k || b.len() != k * n || !self.fits(m, k, n) {
            return None;
        }
        let dev = &self.device;
        let storage_in = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let buf = |contents: &[u8], usage: wgpu::BufferUsages| {
            let b = dev.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: contents.len().max(4) as u64,
                usage,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(&b, 0, contents);
            b
        };
        let a_buf = buf(bytemuck::cast_slice(a), storage_in);
        let b_buf = buf(bytemuck::cast_slice(b), storage_in);
        let mut dims = [0u8; 16];
        dims[0..4].copy_from_slice(&(m as u32).to_le_bytes());
        dims[4..8].copy_from_slice(&(k as u32).to_le_bytes());
        dims[8..12].copy_from_slice(&(n as u32).to_le_bytes());
        let dims_buf = buf(
            &dims,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        let out_bytes = (m * n * 4).max(4) as u64;
        let out = dev.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: out_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = dev.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: out_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: a_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: b_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: dims_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: out.as_entire_binding(),
                },
            ],
        });

        let mut enc = dev.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups((m as u32).div_ceil(8), (n as u32).div_ceil(8), 1);
        }
        enc.copy_buffer_to_buffer(&out, 0, &staging, 0, out_bytes);
        self.queue.submit(Some(enc.finish()));

        let (tx, rx) = mpsc::channel();
        staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        let _ = dev.poll(wgpu::PollType::wait_indefinitely());
        rx.recv().ok()?.ok()?;
        let data = staging
            .slice(..)
            .get_mapped_range()
            .expect("wgpu buffer map_range failed");
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging.unmap();
        Some(result)
    }
}

/// The process-wide GEMM context — built once on first use (lazily), `None` if headless.
/// `device`/`queue`/`pipeline` are `Send + Sync`, so a single context is shared safely.
static GPU_GEMM: OnceLock<Option<WgpuGemm>> = OnceLock::new();

/// Borrow the shared GPU GEMM context, if a GPU is present.
pub fn shared() -> Option<&'static WgpuGemm> {
    GPU_GEMM.get_or_init(WgpuGemm::create).as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CPU reference for the correctness gate.
    fn cpu_gemm(m: usize, k: usize, n: usize, a: &[f32], b: &[f32]) -> Vec<f32> {
        let mut c = vec![0.0f32; m * n];
        for i in 0..m {
            for j in 0..n {
                let mut s = 0.0f32;
                for l in 0..k {
                    s += a[i * k + l] * b[l * n + j];
                }
                c[i * n + j] = s;
            }
        }
        c
    }

    #[test]
    fn substrate_gemm_matches_cpu_reference_when_gpu_present() {
        let Some(ctx) = shared() else {
            eprintln!("[skip] no GPU adapter present — substrate GEMM runs on CPU fallback");
            return;
        };
        eprintln!("[substrate GEMM] GPU = {}", ctx.adapter_label);
        // A non-square case to catch index bugs.
        let (m, k, n) = (33usize, 17usize, 41usize);
        let a: Vec<f32> = (0..m * k).map(|i| ((i % 13) as f32) * 0.1 - 0.5).collect();
        let b: Vec<f32> = (0..k * n).map(|i| ((i % 7) as f32) * 0.2 - 0.3).collect();
        let gpu = ctx.gemm(m, k, n, &a, &b).expect("job fits");
        let cpu = cpu_gemm(m, k, n, &a, &b);
        let max_err = gpu
            .iter()
            .zip(&cpu)
            .map(|(g, c)| (g - c).abs())
            .fold(0.0f32, f32::max);
        assert!(max_err < 1e-3, "GPU vs CPU max abs err {max_err}");
    }
}
