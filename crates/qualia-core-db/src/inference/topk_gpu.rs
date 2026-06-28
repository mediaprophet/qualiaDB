//! STELLAR §A A1a — **native GPU dispatch of the top-K reduction** + on-device parity.
//!
//! Runs `shaders/topk_reduction.wgsl` on a real wgpu device: each workgroup reduces a block of
//! the logits to its top-K candidates, the host merges to the global top-K. The `#[test]` below
//! verifies it **on silicon** against the CPU oracle `topk::topk_cpu` (skips cleanly with no
//! adapter). This is the reusable core that the decode-loop splice (behind `QUALIA_LLM_GPU_TOPK`)
//! will call instead of the full-logit-readback `dispatch_output_argmax_chunked`.
//!
//! Native only — mirrors `ternary_gpu.rs`.

use crate::topk::{
    merge_block_candidates, topk_params_bytes, TopKItem, TOPK_BLOCK_SIZE, TOPK_REDUCTION_WGSL,
};

/// Reduce `logits` to its global top-K on the GPU. `block_size` is elements per workgroup
/// (clamped to `TOPK_BLOCK_SIZE`, the WGSL `var<workgroup>` cap). Blocking (native readback).
pub fn topk_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    logits: &[f32],
    k: usize,
    block_size: usize,
) -> Vec<TopKItem> {
    let n = logits.len();
    let k = k.max(1);
    let block_size = block_size.clamp(1, TOPK_BLOCK_SIZE);
    if n == 0 {
        return Vec::new();
    }
    let num_blocks = n.div_ceil(block_size);
    let cand_count = num_blocks * k;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("topk_reduction"),
        source: wgpu::ShaderSource::Wgsl(TOPK_REDUCTION_WGSL.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("topk_pipeline"),
        layout: None,
        module: &shader,
        entry_point: Some("topk_block"),
        compilation_options: Default::default(),
        cache: None,
    });

    let logits_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("topk_logits"),
        size: (n * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&logits_buf, 0, bytemuck::cast_slice(logits));

    let params = topk_params_bytes(n as u32, k as u32, block_size as u32);
    let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("topk_params"),
        size: params.len() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&params_buf, 0, &params);

    let mk_io = |label: &str| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (cand_count * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    };
    let cand_val = mk_io("topk_cand_val");
    let cand_idx = mk_io("topk_cand_idx");
    let mk_stg = |label: &str| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (cand_count * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        })
    };
    let stg_val = mk_stg("topk_stg_val");
    let stg_idx = mk_stg("topk_stg_idx");

    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("topk_bind"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: logits_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: params_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: cand_val.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: cand_idx.as_entire_binding(),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("topk_enc"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("topk_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups(num_blocks as u32, 1, 1);
    }
    let bytes = (cand_count * 4) as u64;
    encoder.copy_buffer_to_buffer(&cand_val, 0, &stg_val, 0, bytes);
    encoder.copy_buffer_to_buffer(&cand_idx, 0, &stg_idx, 0, bytes);
    queue.submit(Some(encoder.finish()));

    let sv = stg_val.slice(..);
    let si = stg_idx.slice(..);
    let (tx_v, rx_v) = std::sync::mpsc::channel();
    let (tx_i, rx_i) = std::sync::mpsc::channel();
    sv.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx_v.send(r);
    });
    si.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx_i.send(r);
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    rx_v.recv().expect("map val").expect("map topk val");
    rx_i.recv().expect("map idx").expect("map topk idx");

    let dv = sv.get_mapped_range();
    let di = si.get_mapped_range();
    let vals: Vec<f32> = bytemuck::cast_slice(&dv)[..cand_count].to_vec();
    let idxs: Vec<u32> = bytemuck::cast_slice(&di)[..cand_count].to_vec();
    drop(dv);
    drop(di);
    stg_val.unmap();
    stg_idx.unmap();

    merge_block_candidates(&vals, &idxs, k, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topk::topk_cpu;

    fn try_gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .ok()?;
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()
    }

    fn assert_parity(gpu: &[TopKItem], cpu: &[TopKItem]) {
        assert_eq!(gpu.len(), cpu.len(), "top-k length");
        for (g, c) in gpu.iter().zip(cpu.iter()) {
            assert_eq!(
                g.token_id, c.token_id,
                "token id mismatch: gpu {g:?} cpu {c:?}"
            );
            assert!(
                (g.logit - c.logit).abs() < 1e-5,
                "logit mismatch: gpu {g:?} cpu {c:?}"
            );
        }
    }

    /// ON-DEVICE PARITY: small n spanning multiple blocks; top-K == CPU oracle (incl. tie-break).
    #[test]
    fn topk_gpu_matches_cpu_small_multiblock() {
        let Some((device, queue)) = try_gpu() else {
            eprintln!("topk_gpu: no wgpu adapter — skipping");
            return;
        };
        // 50 logits, block_size 16 → 4 workgroups; deterministic with an embedded tie.
        let mut logits: Vec<f32> = (0..50).map(|i| ((i * 13 + 7) % 31) as f32 - 15.0).collect();
        logits[8] = 99.0;
        logits[40] = 99.0; // tie at the top → lower id (8) must win
        logits[3] = f32::NAN; // must never be selected

        for k in [1usize, 5, 12] {
            let gpu = topk_gpu(&device, &queue, &logits, k, 16);
            let cpu = topk_cpu(&logits, k);
            assert_parity(&gpu, &cpu);
        }
        eprintln!("topk_gpu: small multi-block parity OK");
    }

    /// ON-DEVICE PARITY at vocab scale (49 152, block 1024) — the real decode shape.
    #[test]
    fn topk_gpu_matches_cpu_vocab_scale() {
        let Some((device, queue)) = try_gpu() else {
            eprintln!("topk_gpu vocab: no wgpu adapter — skipping");
            return;
        };
        let n = 49_152usize;
        let logits: Vec<f32> = (0..n)
            .map(|i| (((i * 1103515245 + 12345) >> 7) % 1000) as f32 * 0.01 - 5.0)
            .collect();
        for k in [1usize, 32, 64] {
            let gpu = topk_gpu(&device, &queue, &logits, k, TOPK_BLOCK_SIZE);
            let cpu = topk_cpu(&logits, k);
            assert_parity(&gpu, &cpu);
        }
        eprintln!("topk_gpu: vocab-scale (49152) parity OK for k∈{{1,32,64}}");
    }
}
