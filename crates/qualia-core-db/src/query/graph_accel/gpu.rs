//! wgpu dispatch on the process-wide shared device. Any failure returns `None`
//! so the caller uses the CPU floor. Does not create a second adapter.

#![cfg(not(target_arch = "wasm32"))]

use std::sync::OnceLock;

use bytemuck::{Pod, Zeroable};

use crate::gpu_context::try_shared_gpu;
use crate::NQuin;

use super::cpu::QuinField;
use super::path::gpu_available;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RadixParams {
    n: u32,
    shift: u32,
    _pad0: u32,
    _pad1: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SieveParams {
    n: u32,
    field_id: u32,
    match_lo: u32,
    match_hi: u32,
}

struct Kernels {
    hist: wgpu::ComputePipeline,
    scatter: wgpu::ComputePipeline,
    sieve: wgpu::ComputePipeline,
}

fn kernels() -> Option<&'static Kernels> {
    static CELL: OnceLock<Option<Kernels>> = OnceLock::new();
    CELL.get_or_init(|| {
        let gpu = try_shared_gpu()?;
        let device = &gpu.device;
        let hist = pipeline(
            device,
            include_str!("../../shaders/graph_radix_hist.wgsl"),
            "graph-radix-hist",
        );
        let scatter = pipeline(
            device,
            include_str!("../../shaders/graph_radix_scatter.wgsl"),
            "graph-radix-scatter",
        );
        let sieve = pipeline(
            device,
            include_str!("../../shaders/graph_sieve_field.wgsl"),
            "graph-sieve-field",
        );
        Some(Kernels {
            hist,
            scatter,
            sieve,
        })
    })
    .as_ref()
}

fn pipeline(device: &wgpu::Device, src: &str, label: &'static str) -> wgpu::ComputePipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(src.into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    })
}

fn map_read(device: &wgpu::Device, staging: &wgpu::Buffer, size: u64) -> Option<Vec<u8>> {
    let slice = staging.slice(0..size);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    rx.recv().ok()?.ok()?;
    let data = slice.get_mapped_range().ok()?.to_vec();
    staging.unmap();
    Some(data)
}

/// GPU LSD radix of `keys` → permutation `indices` (`indices[i]` is original row).
pub fn radix_sort_u64_indices_gpu(keys: &[u64]) -> Option<Vec<u32>> {
    if !gpu_available() {
        return None;
    }
    let n = keys.len();
    if n == 0 {
        return Some(Vec::new());
    }
    let k = kernels()?;
    let gpu = try_shared_gpu()?;
    let device = &gpu.device;
    let queue = &gpu.queue;

    let mut packed = vec![0u32; n * 2];
    for (i, &x) in keys.iter().enumerate() {
        packed[i * 2] = x as u32;
        packed[i * 2 + 1] = (x >> 32) as u32;
    }
    let idx: Vec<u32> = (0..n as u32).collect();
    let key_bytes = (n * 8) as u64;
    let idx_bytes = (n * 4) as u64;

    let usage_st =
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST;
    let k0 = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("graph-radix-k0"),
        size: key_bytes,
        usage: usage_st,
        mapped_at_creation: false,
    });
    let k1 = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("graph-radix-k1"),
        size: key_bytes,
        usage: usage_st,
        mapped_at_creation: false,
    });
    let i0 = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("graph-radix-i0"),
        size: idx_bytes,
        usage: usage_st,
        mapped_at_creation: false,
    });
    let i1 = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("graph-radix-i1"),
        size: idx_bytes,
        usage: usage_st,
        mapped_at_creation: false,
    });
    queue.write_buffer(&k0, 0, bytemuck::cast_slice(&packed));
    queue.write_buffer(&i0, 0, bytemuck::cast_slice(&idx));

    let hist_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("graph-radix-hist"),
        size: 256 * 4,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let hist_stage = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("graph-radix-hist-stage"),
        size: 256 * 4,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let off_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("graph-radix-off"),
        size: 256 * 4,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let param_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("graph-radix-params"),
        size: std::mem::size_of::<RadixParams>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let wg = (n as u32).div_ceil(256).max(1);
    let mut src_k = &k0;
    let mut src_i = &i0;
    let mut dst_k = &k1;
    let mut dst_i = &i1;

    for shift in (0..64).step_by(8) {
        queue.write_buffer(&hist_buf, 0, &[0u8; 256 * 4]);
        let params = RadixParams {
            n: n as u32,
            shift: shift as u32,
            _pad0: 0,
            _pad1: 0,
        };
        queue.write_buffer(&param_buf, 0, bytemuck::bytes_of(&params));

        let hist_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("graph-radix-hist-bg"),
            layout: &k.hist.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: src_k.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: hist_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: param_buf.as_entire_binding(),
                },
            ],
        });
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("graph-radix-hist-enc"),
        });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("graph-radix-hist"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&k.hist);
            pass.set_bind_group(0, &hist_bg, &[]);
            pass.dispatch_workgroups(wg, 1, 1);
        }
        enc.copy_buffer_to_buffer(&hist_buf, 0, &hist_stage, 0, 256 * 4);
        queue.submit(Some(enc.finish()));
        let raw = map_read(device, &hist_stage, 256 * 4)?;
        let hist: &[u32] = bytemuck::cast_slice(&raw);
        let mut offsets = [0u32; 256];
        let mut sum = 0u32;
        for b in 0..256 {
            offsets[b] = sum;
            sum = sum.saturating_add(hist[b]);
        }
        if sum as usize != n {
            return None;
        }
        queue.write_buffer(&off_buf, 0, bytemuck::cast_slice(&offsets));

        let sc_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("graph-radix-scatter-bg"),
            layout: &k.scatter.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: src_k.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: src_i.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: dst_k.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: dst_i.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: off_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: param_buf.as_entire_binding(),
                },
            ],
        });
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("graph-radix-scatter-enc"),
        });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("graph-radix-scatter"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&k.scatter);
            pass.set_bind_group(0, &sc_bg, &[]);
            pass.dispatch_workgroups(wg, 1, 1);
        }
        queue.submit(Some(enc.finish()));
        std::mem::swap(&mut src_k, &mut dst_k);
        std::mem::swap(&mut src_i, &mut dst_i);
    }

    let idx_stage = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("graph-radix-idx-stage"),
        size: idx_bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("graph-radix-readback"),
    });
    enc.copy_buffer_to_buffer(src_i, 0, &idx_stage, 0, idx_bytes);
    queue.submit(Some(enc.finish()));
    let raw = map_read(device, &idx_stage, idx_bytes)?;
    let idx: Vec<u32> = bytemuck::cast_slice(&raw).to_vec();
    if idx.len() != n {
        return None;
    }
    // Refuse a wrong permutation or unsorted order — caller uses the CPU floor.
    let mut seen = vec![false; n];
    let mut prev = 0u64;
    for (step, &i) in idx.iter().enumerate() {
        let i = i as usize;
        if i >= n || seen[i] {
            return None;
        }
        seen[i] = true;
        let k = keys[i];
        if step > 0 && k < prev {
            return None;
        }
        prev = k;
    }
    Some(idx)
}

pub fn sieve_eq_indices_gpu(quins: &[NQuin], field: QuinField, needle: u64) -> Option<Vec<u32>> {
    if !gpu_available() || quins.is_empty() {
        return None;
    }
    let k = kernels()?;
    let gpu = try_shared_gpu()?;
    let device = &gpu.device;
    let queue = &gpu.queue;
    let n = quins.len();
    let words = n.div_ceil(32);
    let mask_bytes = (words * 4) as u64;
    let quin_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("graph-sieve-quins"),
        size: (n * 48) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&quin_buf, 0, bytemuck::cast_slice(quins));
    let mask_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("graph-sieve-mask"),
        size: mask_bytes,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&mask_buf, 0, &vec![0u8; words * 4]);
    let params = SieveParams {
        n: n as u32,
        field_id: field as u32,
        match_lo: needle as u32,
        match_hi: (needle >> 32) as u32,
    };
    let param_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("graph-sieve-params"),
        size: std::mem::size_of::<SieveParams>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&param_buf, 0, bytemuck::bytes_of(&params));
    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("graph-sieve-bg"),
        layout: &k.sieve.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: quin_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: mask_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: param_buf.as_entire_binding(),
            },
        ],
    });
    let wg = (n as u32).div_ceil(64).max(1);
    let mask_stage = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("graph-sieve-stage"),
        size: mask_bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("graph-sieve-enc"),
    });
    {
        let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("graph-sieve"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&k.sieve);
        pass.set_bind_group(0, &bg, &[]);
        pass.dispatch_workgroups(wg, 1, 1);
    }
    enc.copy_buffer_to_buffer(&mask_buf, 0, &mask_stage, 0, mask_bytes);
    queue.submit(Some(enc.finish()));
    let raw = map_read(device, &mask_stage, mask_bytes)?;
    let words_u32: &[u32] = bytemuck::cast_slice(&raw);
    let mut out = Vec::new();
    for (w, &bits) in words_u32.iter().enumerate() {
        let mut b = bits;
        let base = w * 32;
        while b != 0 {
            let tz = b.trailing_zeros() as usize;
            let idx = base + tz;
            if idx < n {
                out.push(idx as u32);
            }
            b &= b.wrapping_sub(1);
        }
    }
    Some(out)
}
