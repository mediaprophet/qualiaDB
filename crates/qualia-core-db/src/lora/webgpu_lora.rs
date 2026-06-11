//! GPU-accelerated LoRA delta application via wgpu compute shader.
//!
//! Computes `output += B @ (A @ x) * scaling` entirely on the GPU,
//! avoiding a CPU round-trip for the hidden-state delta.
//!
//! # Bind-group layout (group 0)
//!
//! | binding | type            | content                           |
//! |---------|-----------------|-----------------------------------|
//! | 0       | storage rw      | `output` array<f32> (n_out)       |
//! | 1       | storage r       | `input`  array<f32> (n_in)        |
//! | 2       | storage r       | `lora_a` array<f32> (rank × n_in) |
//! | 3       | storage r       | `lora_b` array<f32> (n_out × rank)|
//! | 4       | uniform         | `LoraGpuParams`                   |

use super::adapter_manager::{LoRAAdapter, LoRAError};

// ─── GPU parameter block ─────────────────────────────────────────────────────

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LoraGpuParams {
    pub n_in:    u32,
    pub n_out:   u32,
    pub rank:    u32,
    pub scaling: f32,
}

// ─── LoRAGpuApplicator ────────────────────────────────────────────────────────

/// Holds the compiled wgpu pipeline for LoRA delta computation.
///
/// Constructed once and reused across inference calls.
pub struct LoRAGpuApplicator {
    device:   wgpu::Device,
    queue:    wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bgl:      wgpu::BindGroupLayout,
}

impl LoRAGpuApplicator {
    /// Initialise the wgpu device and compile the LoRA compute shader.
    ///
    /// Synchronous wrapper using `Box::leak` to give a `'static` tokio runtime —
    /// the same pattern used in `QTensorEngine::new()` in `gguf_bridge.rs`.
    pub fn new() -> Result<Self, LoRAError> {
        let rt = Box::leak(Box::new(
            tokio::runtime::Runtime::new()
                .map_err(|e| LoRAError::Io(format!("tokio runtime: {e}")))?,
        ));
        rt.block_on(Self::new_async())
    }

    async fn new_async() -> Result<Self, LoRAError> {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .ok_or_else(|| LoRAError::Io("no wgpu adapter".into()))?;

        // wgpu 0.19: DeviceDescriptor uses Default; features/limits are fields of the
        // inner `wgpu::DeviceDescriptor::default()` but the public struct in this version
        // is just `&wgpu::DeviceDescriptor::default()`.
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .map_err(|e| LoRAError::Io(format!("wgpu device: {e}")))?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label:  Some("lora_apply"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/lora_apply.wgsl").into(),
            ),
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("lora-bgl"),
            entries: &[
                storage_rw_entry(0), // output
                storage_r_entry(1),  // input
                storage_r_entry(2),  // lora_a
                storage_r_entry(3),  // lora_b
                uniform_entry(4),    // params
            ],
        });

        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label:                Some("lora-pl"),
            bind_group_layouts:   &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label:       Some("lora-pipeline"),
            layout:      Some(&pl),
            module:      &shader,
            entry_point: "apply_lora",
        });

        Ok(Self { device, queue, pipeline, bgl })
    }

    /// Apply the LoRA delta to `output` on the GPU.
    ///
    /// `input`  — hidden state, length `n_in`
    /// `output` — receives `+= B @ (A @ input) * scaling`, length `n_out`
    ///
    /// On any GPU error returns `Err`; caller should fall back to `LoRAAdapter::apply_cpu`.
    pub fn apply(
        &self,
        adapter: &LoRAAdapter,
        input:   &[f32],
        output:  &mut [f32],
    ) -> Result<(), LoRAError> {
        let n_in    = adapter.meta.n_in;
        let n_out   = adapter.meta.n_out;
        let rank    = adapter.meta.rank as usize;
        let scaling = adapter.meta.scaling();

        if input.len() != n_in || output.len() != n_out {
            return Err(LoRAError::InferenceDimMismatch {
                input_len: input.len(),
                lora_n_in: n_in,
            });
        }

        let params = LoraGpuParams {
            n_in: n_in as u32, n_out: n_out as u32, rank: rank as u32, scaling,
        };

        let dev = &self.device;

        // ── Upload buffers ────────────────────────────────────────────────────
        let out_buf    = buf_rw(dev, bytemuck::cast_slice(output));
        let in_buf     = buf_r(dev, bytemuck::cast_slice(input));
        let a_buf      = buf_r(dev, bytemuck::cast_slice(&adapter.lora_a.data));
        let b_buf      = buf_r(dev, bytemuck::cast_slice(&adapter.lora_b.data));
        let params_buf = buf_uniform(dev, bytemuck::bytes_of(&params));

        let out_staging = dev.create_buffer(&wgpu::BufferDescriptor {
            label:              Some("lora-out-staging"),
            size:               (n_out * 4) as u64,
            usage:              wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── Bind group ────────────────────────────────────────────────────────
        let bg = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("lora-bg"),
            layout:  &self.bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: out_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: in_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: a_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: b_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: params_buf.as_entire_binding() },
            ],
        });

        // ── Dispatch ─────────────────────────────────────────────────────────
        let mut enc = dev.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("lora-enc"),
        });

        {
            let mut cpass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label:            Some("lora-pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &bg, &[]);
            cpass.dispatch_workgroups((n_out as u32 + 63) / 64, 1, 1);
        }

        enc.copy_buffer_to_buffer(&out_buf, 0, &out_staging, 0, (n_out * 4) as u64);
        self.queue.submit(Some(enc.finish()));

        // ── Readback ─────────────────────────────────────────────────────────
        let slice = out_staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        dev.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|_| LoRAError::Io("GPU readback channel closed".into()))?
            .map_err(|e| LoRAError::Io(format!("map_async: {e:?}")))?;

        let mapped = slice.get_mapped_range();
        let result: &[f32] = bytemuck::cast_slice(&mapped);
        output.copy_from_slice(result);
        Ok(())
    }
}

// ─── wgpu buffer helpers ─────────────────────────────────────────────────────

fn buf_rw(dev: &wgpu::Device, data: &[u8]) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    dev.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label:    Some("lora-rw"),
        contents: data,
        usage:    wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
    })
}

fn buf_r(dev: &wgpu::Device, data: &[u8]) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    dev.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label:    Some("lora-r"),
        contents: data,
        usage:    wgpu::BufferUsages::STORAGE,
    })
}

fn buf_uniform(dev: &wgpu::Device, data: &[u8]) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    dev.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label:    Some("lora-uniform"),
        contents: data,
        usage:    wgpu::BufferUsages::UNIFORM,
    })
}

fn storage_rw_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty:                 wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size:   None,
        },
        count: None,
    }
}

fn storage_r_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty:                 wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size:   None,
        },
        count: None,
    }
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty:                 wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size:   None,
        },
        count: None,
    }
}
