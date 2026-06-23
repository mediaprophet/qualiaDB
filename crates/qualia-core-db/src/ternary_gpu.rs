//! Task #12 / STELLAR §A — **native GPU dispatch of the ternary GEMM** + on-device parity.
//!
//! Runs `shaders/ternary_gemm.wgsl` (`ternary::TERNARY_GEMM_WGSL`) on a real wgpu device. This is
//! the piece the FFN inference loop calls to execute a ternary-packed weight on the GPU; the
//! `#[test]` below verifies it **on silicon** against the byte-exact CPU oracle
//! `ternary::ternary_gemm_cpu` (it skips cleanly when no adapter is present, e.g. headless CI).
//!
//! Native only — the wasm WebGPU path reuses the same WGSL through `gguf_bridge`'s pipeline set
//! when the kernel is spliced into the layer loop (the remaining integration step).

use crate::ternary::TERNARY_GEMM_WGSL;

/// 32-byte `TernaryParams` uniform matching `ternary_gemm.wgsl` (n_in, n_out, n_batch,
/// in_row_stride, out_row_stride, scale, + 2 pad words).
fn ternary_params_bytes(n_in: u32, n_out: u32, n_batch: u32, scale: f32) -> [u8; 32] {
    let mut b = [0u8; 32];
    b[0..4].copy_from_slice(&n_in.to_le_bytes());
    b[4..8].copy_from_slice(&n_out.to_le_bytes());
    b[8..12].copy_from_slice(&n_batch.to_le_bytes());
    // in_row_stride / out_row_stride = 0 → dense (the shader falls back to n_in / n_out)
    b[20..24].copy_from_slice(&scale.to_le_bytes());
    b
}

/// Execute the ternary GEMM on the GPU: returns the `n_batch × n_out` output (row-major).
/// `packed` is the row-major trits of the `(n_out × n_in)` weight (5/byte base-3). Strides default
/// to dense. Blocking (native readback).
pub fn ternary_gemm_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    activations: &[f32],
    packed: &[u8],
    scale: f32,
    n_in: usize,
    n_out: usize,
    n_batch: usize,
) -> Vec<f32> {
    let n_batch = n_batch.max(1);
    let out_elems = n_batch * n_out;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ternary_gemm"),
        source: wgpu::ShaderSource::Wgsl(TERNARY_GEMM_WGSL.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("ternary_gemm_pipeline"),
        layout: None, // auto bind-group layout (native)
        module: &shader,
        entry_point: "ternary_gemm",
        compilation_options: Default::default(),
    });

    let mk_buf = |label: &str, contents: &[u8], usage: wgpu::BufferUsages| {
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: contents.len().max(4) as u64,
            usage,
            mapped_at_creation: false,
        });
        if !contents.is_empty() {
            queue.write_buffer(&buf, 0, contents);
        }
        buf
    };

    let act_buf = mk_buf(
        "ternary_act",
        bytemuck::cast_slice(activations),
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    );
    // trit_words is array<u32> → pad the packed bytes to a 4-byte multiple.
    let mut trits = packed.to_vec();
    while trits.len() % 4 != 0 || trits.is_empty() {
        trits.push(0);
    }
    let trit_buf = mk_buf("ternary_trits", &trits, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST);
    let params = ternary_params_bytes(n_in as u32, n_out as u32, n_batch as u32, scale);
    let param_buf = mk_buf("ternary_params", &params, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
    let out_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ternary_out"),
        size: (out_elems * 4).max(4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ternary_staging"),
        size: (out_elems * 4).max(4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ternary_bind"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: act_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: trit_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: param_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: out_buf.as_entire_binding() },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("ternary_enc") });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("ternary_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        let wg_x = (n_out as u32).div_ceil(64).max(1);
        pass.dispatch_workgroups(wg_x, n_batch as u32, 1);
    }
    encoder.copy_buffer_to_buffer(&out_buf, 0, &staging, 0, (out_elems * 4).max(4) as u64);
    queue.submit(Some(encoder.finish()));

    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    let _ = device.poll(wgpu::Maintain::Wait);
    rx.recv().expect("map channel").expect("map ternary staging");
    let data = slice.get_mapped_range();
    let out: Vec<f32> = bytemuck::cast_slice(&data)[..out_elems].to_vec();
    drop(data);
    staging.unmap();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ternary::{pack_trits, ternary_gemm_cpu};

    fn try_gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))?;
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None)).ok()
    }

    /// ON-DEVICE PARITY: `ternary_gemm.wgsl` on a real GPU == `ternary_gemm_cpu`. Skips cleanly with
    /// no adapter (headless CI); runs for real on a machine with a GPU.
    #[test]
    fn ternary_gemm_gpu_matches_cpu_oracle() {
        let Some((device, queue)) = try_gpu() else {
            eprintln!("ternary_gemm_gpu: no wgpu adapter — skipping on-device parity");
            return;
        };

        // (n_out=5 × n_in=7), 3 batch rows; deterministic trits + activations.
        let (n_in, n_out, n_batch) = (7usize, 5usize, 3usize);
        let scale = 0.37_f32;
        let trits: Vec<i8> = (0..n_in * n_out).map(|k| (k % 3) as i8 - 1).collect();
        let packed = pack_trits(&trits);
        let act: Vec<f32> = (0..n_in * n_batch).map(|j| (j as f32) * 0.25 - 1.5).collect();

        let gpu = ternary_gemm_gpu(&device, &queue, &act, &packed, scale, n_in, n_out, n_batch);

        let mut cpu = vec![0.0f32; n_batch * n_out];
        ternary_gemm_cpu(&act, &packed, scale, n_in, n_out, n_batch, 0, 0, &mut cpu);

        assert_eq!(gpu.len(), cpu.len());
        for i in 0..cpu.len() {
            assert!((gpu[i] - cpu[i]).abs() < 1e-4, "elem {i}: gpu {} vs cpu {}", gpu[i], cpu[i]);
        }
        eprintln!("ternary_gemm_gpu: on-device parity OK ({} elems)", cpu.len());
    }
}
