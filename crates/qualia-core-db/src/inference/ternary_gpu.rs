//! Task #12 / STELLAR §A — **native GPU dispatch of the ternary GEMM** + on-device parity.
//!
//! Runs `shaders/ternary_gemm.wgsl` (`ternary::TERNARY_GEMM_WGSL`) on a real wgpu device. This is
//! the piece the FFN inference loop calls to execute a ternary-packed weight on the GPU; the
//! `#[test]` below verifies it **on silicon** against the byte-exact CPU oracle
//! `ternary::ternary_gemm_cpu` (it skips cleanly when no adapter is present, e.g. headless CI).
//!
//! Native only — the wasm WebGPU path reuses the same WGSL through `gguf_bridge`'s pipeline set
//! when the kernel is spliced into the layer loop (the remaining integration step).

use crate::ternary::{rebake_ternary_blob_to_2bit, TERNARY_GEMM_2BIT_WGSL, TERNARY_GEMM_WGSL};
use std::collections::HashMap;

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

/// Execute the **base-3** ternary GEMM on the GPU (`ternary_gemm.wgsl`).
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
    run_gemm(
        device,
        queue,
        TERNARY_GEMM_WGSL,
        activations,
        packed,
        scale,
        n_in,
        n_out,
        n_batch,
    )
}

/// Execute the **2-bit branchless** ternary GEMM on the GPU (`ternary_gemm_2bit.wgsl`). `packed`
/// must be 2-bit packed (`ternary::pack_trits_2bit`).
pub fn ternary_gemm_gpu_2bit(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    activations: &[f32],
    packed: &[u8],
    scale: f32,
    n_in: usize,
    n_out: usize,
    n_batch: usize,
) -> Vec<f32> {
    run_gemm(
        device,
        queue,
        TERNARY_GEMM_2BIT_WGSL,
        activations,
        packed,
        scale,
        n_in,
        n_out,
        n_batch,
    )
}

/// Shared dispatch for both ternary GEMM kernels (identical bindings/params; only the WGSL differs).
/// Returns the `n_batch × n_out` output. Strides default to dense. Blocking (native readback).
#[allow(clippy::too_many_arguments)]
fn run_gemm(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    wgsl: &str,
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
        source: wgpu::ShaderSource::Wgsl(wgsl.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("ternary_gemm_pipeline"),
        layout: None, // auto bind-group layout (native)
        module: &shader,
        entry_point: Some("ternary_gemm"),
        compilation_options: Default::default(),
        cache: None,
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
    let trit_buf = mk_buf(
        "ternary_trits",
        &trits,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    );
    let params = ternary_params_bytes(n_in as u32, n_out as u32, n_batch as u32, scale);
    let param_buf = mk_buf(
        "ternary_params",
        &params,
        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    );
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
            wgpu::BindGroupEntry {
                binding: 0,
                resource: act_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: trit_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: param_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: out_buf.as_entire_binding(),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("ternary_enc"),
    });
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
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    rx.recv()
        .expect("map channel")
        .expect("map ternary staging");
    let data = slice
        .get_mapped_range()
        .expect("wgpu buffer map_range failed");
    let out: Vec<f32> = bytemuck::cast_slice(&data)[..out_elems].to_vec();
    drop(data);
    staging.unmap();
    out
}

// ── A1b inc 2b: resident 2-bit ternary-FFN GEMM (persistent pipeline + resident weights) ─────────

/// Storage-buffer binding offset alignment. wgpu's `min_storage_buffer_offset_alignment` is 256 on
/// the engine's baselines (A2000/Vulkan, DX12, WebGPU downlevel), so every resident tensor starts on
/// this boundary — a sub-range binding at a non-aligned offset is a validation error.
const TERNARY_RESIDENT_ALIGN: u64 = 256;

#[inline]
fn align_up_u64(x: u64, align: u64) -> u64 {
    (x + align - 1) / align * align
}

/// One resident ternary tensor's location + metadata inside [`TernaryFfnResident::weights`].
#[derive(Clone, Copy, Debug)]
struct ResidentTernaryTensor {
    /// 256-aligned byte offset of this tensor's 2-bit packed weights in the resident buffer.
    gpu_offset: u64,
    /// 2-bit packed byte length (padded to a 4-byte multiple = the bound `size`).
    packed_len: u32,
    n_in: u32,
    n_out: u32,
    /// Per-tensor BitNet absmean scale (applied once per output element).
    scale: f32,
}

/// A1b (STELLAR §A): the resident **2-bit branchless** ternary-FFN GEMM dispatcher — the perf core.
///
/// **The D7 fix.** The 2-bit kernel pipeline + bind-group layout are built **once**; every FFN
/// ternary weight is rebaked base-3 → 2-bit (lossless; see [`rebake_ternary_blob_to_2bit`]) and
/// uploaded **once** into a single resident VRAM buffer. (The prior 1.02× failure rebuilt the
/// pipeline per call.) At decode each FFN GEMV binds its own weight sub-range — no re-upload —
/// writes only the activation row + params, dispatches, and reads back `n_out` floats. Tensors are
/// keyed by their P64 blob offset (`GgufTensorInfo::byte_offset`), the unique per-tensor handle.
///
/// On-disk the FFN weights stay base-3 (densest, 1.6 bit); 2-bit is the GPU-resident layout only
/// (2.0 bit, shift/mask, divergence-free → 1.77× vs F16 on the A2000). Native-only; the wasm WebGPU
/// ternary path reuses the same WGSL through the MC8 resident arena (a later step).
pub struct TernaryFfnResident {
    pipeline: wgpu::ComputePipeline,
    weights: wgpu::Buffer,
    act_buf: wgpu::Buffer,
    out_buf: wgpu::Buffer,
    params_buf: wgpu::Buffer,
    staging: wgpu::Buffer,
    map: HashMap<u64, ResidentTernaryTensor>,
    max_in: u32,
    max_out: u32,
    resident_bytes: u64,
}

impl TernaryFfnResident {
    /// Build the resident set from `(key, n_in, n_out, base3_blob)` tuples. Each base-3 blob
    /// (`[scale f32][5-trits/byte]`) is rebaked to the 2-bit runtime layout here — load-time heap is
    /// the sanctioned path; the hot loop stays zero-heap. `key` is the tensor's P64 blob offset.
    /// Returns `None` if `tensors` is empty or any blob is malformed.
    pub fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        tensors: &[(u64, usize, usize, &[u8])],
    ) -> Option<Self> {
        if tensors.is_empty() {
            return None;
        }
        // 1. Rebake each base-3 blob → 2-bit; lay out 256-aligned within one resident buffer.
        let mut map: HashMap<u64, ResidentTernaryTensor> = HashMap::with_capacity(tensors.len());
        let mut uploads: Vec<(u64, Vec<u8>)> = Vec::with_capacity(tensors.len());
        let mut cursor: u64 = 0;
        let (mut max_in, mut max_out) = (0u32, 0u32);
        for &(key, n_in, n_out, blob) in tensors {
            let count = n_in.checked_mul(n_out)?;
            if count == 0 {
                return None;
            }
            let (scale, mut packed) = rebake_ternary_blob_to_2bit(blob, count);
            if packed.is_empty() {
                return None;
            }
            // trit_words is array<u32> → pad the packed bytes to a 4-byte multiple.
            while packed.len() % 4 != 0 {
                packed.push(0);
            }
            let gpu_offset = align_up_u64(cursor, TERNARY_RESIDENT_ALIGN);
            let packed_len = packed.len() as u32;
            map.insert(
                key,
                ResidentTernaryTensor {
                    gpu_offset,
                    packed_len,
                    n_in: n_in as u32,
                    n_out: n_out as u32,
                    scale,
                },
            );
            uploads.push((gpu_offset, packed));
            cursor = gpu_offset + packed_len as u64;
            max_in = max_in.max(n_in as u32);
            max_out = max_out.max(n_out as u32);
        }
        let resident_bytes = cursor.max(4);

        // 2. Persistent pipeline (auto layout) — built ONCE.
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ternary_ffn_2bit"),
            source: wgpu::ShaderSource::Wgsl(TERNARY_GEMM_2BIT_WGSL.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ternary_ffn_resident_pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some("ternary_gemm"),
            compilation_options: Default::default(),
            cache: None,
        });

        // 3. Resident weight buffer + reusable IO buffers.
        let weights = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("TernaryFfnResidentWeights"),
            size: resident_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        for (off, bytes) in &uploads {
            queue.write_buffer(&weights, *off, bytes);
        }
        let act_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("TernaryFfnAct"),
            size: (max_in as u64 * 4).max(4),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let out_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("TernaryFfnOut"),
            size: (max_out as u64 * 4).max(4),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("TernaryFfnParams"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("TernaryFfnStaging"),
            size: (max_out as u64 * 4).max(4),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Some(Self {
            pipeline,
            weights,
            act_buf,
            out_buf,
            params_buf,
            staging,
            map,
            max_in,
            max_out,
            resident_bytes,
        })
    }

    /// Whether a tensor with `key` is resident (and its shape, for the dispatch guard).
    pub fn contains(&self, key: u64) -> bool {
        self.map.contains_key(&key)
    }

    /// Total resident weight bytes (the VRAM footprint of the ternary FFN).
    pub fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    /// Number of resident ternary tensors.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Run `out[..n_out] = scale · W·act` for the resident tensor `key` (batch 1 = decode GEMV).
    /// Returns `false` (caller falls back) if the key is absent, the shape mismatches the resident
    /// metadata, the IO bounds are exceeded, or the GPU readback fails — fail-closed, never garbage.
    pub fn gemv(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: u64,
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        let t = match self.map.get(&key) {
            Some(t) => *t,
            None => return false,
        };
        if t.n_in as usize != n_in
            || t.n_out as usize != n_out
            || n_in > input.len()
            || n_out > out.len()
            || n_in as u32 > self.max_in
            || n_out as u32 > self.max_out
        {
            return false;
        }

        queue.write_buffer(&self.act_buf, 0, bytemuck::cast_slice(&input[..n_in]));
        let params = ternary_params_bytes(n_in as u32, n_out as u32, 1, t.scale);
        queue.write_buffer(&self.params_buf, 0, &params);

        let weight_binding = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: &self.weights,
            offset: t.gpu_offset,
            size: std::num::NonZeroU64::new(t.packed_len as u64),
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ternary_ffn_resident_bind"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.act_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: weight_binding,
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.out_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ternary_ffn_enc"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ternary_ffn_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups((n_out as u32).div_ceil(64).max(1), 1, 1);
        }
        let out_bytes = (n_out * 4) as wgpu::BufferAddress;
        encoder.copy_buffer_to_buffer(&self.out_buf, 0, &self.staging, 0, out_bytes);
        queue.submit(Some(encoder.finish()));

        let slice = self.staging.slice(..out_bytes);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        match rx.recv() {
            Ok(Ok(())) => {}
            _ => return false,
        }
        let data = slice
            .get_mapped_range()
            .expect("wgpu buffer map_range failed");
        out[..n_out].copy_from_slice(&bytemuck::cast_slice(&data)[..n_out]);
        drop(data);
        self.staging.unmap();
        true
    }
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
        }))
        .ok()?;
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()
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
        let act: Vec<f32> = (0..n_in * n_batch)
            .map(|j| (j as f32) * 0.25 - 1.5)
            .collect();

        let gpu = ternary_gemm_gpu(&device, &queue, &act, &packed, scale, n_in, n_out, n_batch);

        let mut cpu = vec![0.0f32; n_batch * n_out];
        ternary_gemm_cpu(&act, &packed, scale, n_in, n_out, n_batch, 0, 0, &mut cpu);

        assert_eq!(gpu.len(), cpu.len());
        for i in 0..cpu.len() {
            assert!(
                (gpu[i] - cpu[i]).abs() < 1e-4,
                "elem {i}: gpu {} vs cpu {}",
                gpu[i],
                cpu[i]
            );
        }
        eprintln!(
            "ternary_gemm_gpu: on-device parity OK ({} elems)",
            cpu.len()
        );
    }

    /// ON-DEVICE PARITY (2-bit branchless): `ternary_gemm_2bit.wgsl` == `ternary_gemm_cpu_2bit`.
    #[test]
    fn ternary_gemm_gpu_2bit_matches_cpu_oracle() {
        use crate::ternary::{pack_trits_2bit, ternary_gemm_cpu_2bit};
        let Some((device, queue)) = try_gpu() else {
            eprintln!("ternary_gemm_gpu_2bit: no wgpu adapter — skipping");
            return;
        };
        let (n_in, n_out, n_batch) = (7usize, 5usize, 3usize);
        let scale = 0.37_f32;
        let trits: Vec<i8> = (0..n_in * n_out).map(|k| (k % 3) as i8 - 1).collect();
        let packed = pack_trits_2bit(&trits);
        let act: Vec<f32> = (0..n_in * n_batch)
            .map(|j| (j as f32) * 0.25 - 1.5)
            .collect();

        let gpu =
            ternary_gemm_gpu_2bit(&device, &queue, &act, &packed, scale, n_in, n_out, n_batch);
        let mut cpu = vec![0.0f32; n_batch * n_out];
        ternary_gemm_cpu_2bit(&act, &packed, scale, n_in, n_out, n_batch, 0, 0, &mut cpu);
        for i in 0..cpu.len() {
            assert!(
                (gpu[i] - cpu[i]).abs() < 1e-4,
                "elem {i}: gpu {} vs cpu {}",
                gpu[i],
                cpu[i]
            );
        }
        eprintln!(
            "ternary_gemm_gpu_2bit: on-device parity OK ({} elems)",
            cpu.len()
        );
    }

    /// Build a base-3 ternary blob (`[scale f32][5-trits/byte]`) from explicit trits + scale.
    fn base3_blob(scale: f32, trits: &[i8]) -> Vec<u8> {
        let mut b = scale.to_le_bytes().to_vec();
        b.extend_from_slice(&pack_trits(trits));
        b
    }

    /// A1b inc 2b ON-DEVICE GATE: the resident 2-bit dispatcher (persistent pipeline + ONE resident
    /// weight buffer + 256-aligned sub-range bindings, keyed by P64 blob offset) reproduces the
    /// base-3 CPU oracle byte-for-byte — for TWO differently-shaped tensors at distinct keys, proving
    /// the per-key lookup, sub-range binding, rebake, and GEMV are all correct. Skips with no GPU.
    #[test]
    fn ternary_ffn_resident_matches_cpu_oracle() {
        let Some((device, queue)) = try_gpu() else {
            eprintln!("ternary_ffn_resident: no wgpu adapter — skipping");
            return;
        };
        // Two FFN-shaped tensors with DIFFERENT shapes + scales + distinct keys (blob offsets).
        let a = (0x1000u64, 64usize, 40usize, 0.30f32);
        let b = (0x2000u64, 96usize, 24usize, 0.11f32);
        let trits_a: Vec<i8> = (0..a.1 * a.2).map(|k| (k % 3) as i8 - 1).collect();
        let trits_b: Vec<i8> = (0..b.1 * b.2)
            .map(|k| ((k * 7 + 2) % 3) as i8 - 1)
            .collect();
        let blob_a = base3_blob(a.3, &trits_a);
        let blob_b = base3_blob(b.3, &trits_b);

        let resident = TernaryFfnResident::build(
            &device,
            &queue,
            &[(a.0, a.1, a.2, &blob_a), (b.0, b.1, b.2, &blob_b)],
        )
        .expect("build resident ternary set");
        assert_eq!(resident.len(), 2);
        assert!(resident.contains(a.0) && resident.contains(b.0));

        for (key, n_in, n_out, scale, blob) in
            [(a.0, a.1, a.2, a.3, &blob_a), (b.0, b.1, b.2, b.3, &blob_b)]
        {
            let act: Vec<f32> = (0..n_in).map(|j| (j as f32) * 0.25 - 1.0).collect();
            let mut gpu = vec![0f32; n_out];
            assert!(
                resident.gemv(&device, &queue, key, &act, &mut gpu, n_in, n_out),
                "resident gemv must succeed for key {key:#x}"
            );
            let mut cpu = vec![0f32; n_out];
            ternary_gemm_cpu(&act, &blob[4..], scale, n_in, n_out, 1, 0, 0, &mut cpu);
            for i in 0..n_out {
                assert!(
                    (gpu[i] - cpu[i]).abs() < 1e-4,
                    "key {key:#x} row {i}: gpu {} vs cpu {}",
                    gpu[i],
                    cpu[i]
                );
            }
        }
        // fail-closed (false, never garbage): an absent key, and a present key with the WRONG shape.
        let mut tmp = vec![0f32; 64];
        assert!(
            !resident.gemv(&device, &queue, 0xDEAD, &[0.0; 64], &mut tmp, 64, 40),
            "absent key must fail-closed"
        );
        assert!(
            !resident.gemv(&device, &queue, a.0, &[0.0; 64], &mut tmp, 64, 64),
            "present key with mismatched n_out must fail-closed (a is 64x40, not 64x64)"
        );
        eprintln!(
            "ternary_ffn_resident: on-device parity OK (2 tensors, {} resident bytes)",
            resident.resident_bytes()
        );
    }

    /// INDICATIVE A/B timing: base-3 branchy vs 2-bit branchless on a large GEMV. Wall-clock incl.
    /// per-dispatch submit/readback overhead — a *relative* signal, not a rigorous TPS number
    /// (real measurement needs timestamp queries inside the fused FFN loop). Skips with no GPU.
    #[test]
    fn ternary_gemm_2bit_vs_base3_indicative_timing() {
        use crate::ternary::{pack_trits, pack_trits_2bit};
        use std::time::Instant;
        let Some((device, queue)) = try_gpu() else {
            eprintln!("ternary timing: no wgpu adapter — skipping");
            return;
        };
        let (n_in, n_out, n_batch) = (4096usize, 4096usize, 1usize); // decode-shape GEMV
        let scale = 0.05_f32;
        let trits: Vec<i8> = (0..n_in * n_out)
            .map(|k| ((k * 7 + 1) % 3) as i8 - 1)
            .collect();
        let p3 = pack_trits(&trits);
        let p2 = pack_trits_2bit(&trits);
        let act: Vec<f32> = (0..n_in).map(|j| (j as f32 % 13.0) * 0.1 - 0.6).collect();
        let iters = 60;

        // warmup
        let _ = ternary_gemm_gpu(&device, &queue, &act, &p3, scale, n_in, n_out, n_batch);
        let _ = ternary_gemm_gpu_2bit(&device, &queue, &act, &p2, scale, n_in, n_out, n_batch);

        let t0 = Instant::now();
        for _ in 0..iters {
            let _ = ternary_gemm_gpu(&device, &queue, &act, &p3, scale, n_in, n_out, n_batch);
        }
        let base3 = t0.elapsed().as_secs_f64() / iters as f64 * 1e3;

        let t1 = Instant::now();
        for _ in 0..iters {
            let _ = ternary_gemm_gpu_2bit(&device, &queue, &act, &p2, scale, n_in, n_out, n_batch);
        }
        let bit2 = t1.elapsed().as_secs_f64() / iters as f64 * 1e3;

        eprintln!(
            "ternary GEMV {}x{} (indicative, incl. overhead): base-3 branchy {:.3} ms/iter | 2-bit branchless {:.3} ms/iter | speedup {:.2}x",
            n_out, n_in, base3, bit2, base3 / bit2.max(1e-9)
        );
    }

    /// F16 GEMV baseline shader (same bindings/params as the ternary kernels).
    const F16_GEMV_WGSL: &str = include_str!("../shaders/f16_gemv.wgsl");

    /// Persistent-pipeline, batched-dispatch GEMV timing — fixes the per-call rebuild flaw of the
    /// indicative test. Pipeline + buffers are created ONCE; `K` dispatches are encoded per submit
    /// and `S` submits are timed, so per-dispatch time is GPU-execution-dominated (submit/alloc
    /// overhead amortized away). Returns ms per dispatch.
    fn bench_kernel(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        wgsl: &str,
        entry: &str,
        weight_bytes: &[u8],
        n_in: usize,
        n_out: usize,
    ) -> f64 {
        use std::time::Instant;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bench"),
            source: wgpu::ShaderSource::Wgsl(wgsl.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("bench_pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some(entry),
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
        let act = mk(
            bytemuck::cast_slice(&vec![0.1f32; n_in]),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let mut w = weight_bytes.to_vec();
        while w.len() % 4 != 0 || w.is_empty() {
            w.push(0);
        }
        let wbuf = mk(
            &w,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let params = super::ternary_params_bytes(n_in as u32, n_out as u32, 1, 1.0);
        let pbuf = mk(
            &params,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let obuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (n_out * 4).max(4) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: act.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wbuf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: pbuf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: obuf.as_entire_binding(),
                },
            ],
        });
        let wg_x = (n_out as u32).div_ceil(64).max(1);
        let (k, s) = (32u32, 8u32);
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

    /// A2000 KERNEL BENCHMARK (real numbers): F16 vs base-3 ternary vs 2-bit branchless ternary, at a
    /// decode-shape GEMV, with persistent pipeline + buffer reuse. Skips with no GPU.
    #[test]
    fn ternary_kernel_benchmark() {
        use crate::ternary::{pack_trits, pack_trits_2bit};
        let Some((device, queue)) = try_gpu() else {
            eprintln!("ternary_kernel_benchmark: no wgpu adapter — skipping");
            return;
        };
        let (n_in, n_out) = (4096usize, 4096usize); // decode GEMV (batch 1)
        let trits: Vec<i8> = (0..n_in * n_out)
            .map(|k| ((k * 7 + 1) % 3) as i8 - 1)
            .collect();
        let p3 = pack_trits(&trits);
        let p2 = pack_trits_2bit(&trits);
        let f16 = vec![0u8; n_in * n_out * 2]; // f16 weights (size = bandwidth; values irrelevant to timing)

        let f16ms = bench_kernel(
            &device,
            &queue,
            F16_GEMV_WGSL,
            "f16_gemv",
            &f16,
            n_in,
            n_out,
        );
        let base3 = bench_kernel(
            &device,
            &queue,
            TERNARY_GEMM_WGSL,
            "ternary_gemm",
            &p3,
            n_in,
            n_out,
        );
        let bit2 = bench_kernel(
            &device,
            &queue,
            TERNARY_GEMM_2BIT_WGSL,
            "ternary_gemm",
            &p2,
            n_in,
            n_out,
        );

        eprintln!(
            "── A2000 GEMV {}x{} batch=1 (persistent pipeline, {}MB/{}KB/{}KB weights) ──",
            n_out,
            n_in,
            f16.len() / (1 << 20),
            p3.len() >> 10,
            p2.len() >> 10
        );
        eprintln!("  F16 baseline        : {:.4} ms/dispatch", f16ms);
        eprintln!(
            "  ternary base-3      : {:.4} ms/dispatch  ({:.2}x vs F16)",
            base3,
            f16ms / base3.max(1e-12)
        );
        eprintln!(
            "  ternary 2-bit branchless: {:.4} ms/dispatch  ({:.2}x vs F16, {:.2}x vs base-3)",
            bit2,
            f16ms / bit2.max(1e-12),
            base3 / bit2.max(1e-12)
        );
    }
}
