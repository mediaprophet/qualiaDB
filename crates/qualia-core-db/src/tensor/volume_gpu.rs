//! GPU tensor volume search (B8) — mirrors `visit_tensor_search_into` on U1 queue lane.

use bytemuck::{Pod, Zeroable};

use super::Tensor10D;
use super::resident_substrate::{global_resident_substrate, MAX_KNN_HITS, MAX_RESIDENT_NODES};

pub const TENSOR_VOLUME_STRIDE_FLOATS: u32 = 10;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct VolumeGpuParams {
    node_count: u32,
    max_distance: f32,
    stride_floats: u32,
    max_hits: u32,
}

#[cfg(not(target_arch = "wasm32"))]
pub struct TensorVolumeGpu {
    pipeline: wgpu::ComputePipeline,
    query_buf: wgpu::Buffer,
    nodes_buf: wgpu::Buffer,
    params_buf: wgpu::Buffer,
    hits_buf: wgpu::Buffer,
    count_buf: wgpu::Buffer,
    staging_hits: wgpu::Buffer,
    staging_count: wgpu::Buffer,
    max_nodes: u32,
}

#[cfg(not(target_arch = "wasm32"))]
impl TensorVolumeGpu {
    pub fn try_new(device: &wgpu::Device) -> Option<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("TensorVolumeShader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/tensor_volume.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("TensorVolumePipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
        });
        let max_nodes = MAX_RESIDENT_NODES as u32;
        let node_floats = max_nodes * TENSOR_VOLUME_STRIDE_FLOATS;
        Some(Self {
            pipeline,
            query_buf: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("TensorVolumeQuery"),
                size: std::mem::size_of::<Tensor10D>() as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            nodes_buf: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("TensorVolumeNodes"),
                size: (node_floats as usize * 4) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            params_buf: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("TensorVolumeParams"),
                size: std::mem::size_of::<VolumeGpuParams>() as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            hits_buf: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("TensorVolumeHits"),
                size: (MAX_KNN_HITS * 4) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            count_buf: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("TensorVolumeHitCount"),
                size: 4,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            staging_hits: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("TensorVolumeHitsStaging"),
                size: (MAX_KNN_HITS * 4) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            staging_count: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("TensorVolumeCountStaging"),
                size: 4,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            max_nodes,
        })
    }

    fn upload_nodes(&self, queue: &wgpu::Queue, node_count: u32) -> bool {
        let substrate = global_resident_substrate();
        let count = node_count.min(substrate.node_count());
        if count == 0 {
            return false;
        }
        let mut flat = [0f32; MAX_RESIDENT_NODES * TENSOR_VOLUME_STRIDE_FLOATS as usize];
        for i in 0..count as usize {
            if let Some(t) = substrate.tensor_at(i as u32) {
                let base = i * TENSOR_VOLUME_STRIDE_FLOATS as usize;
                flat[base] = t.q;
                flat[base + 1] = t.v;
                flat[base + 2] = t.w;
                flat[base + 3] = t.x;
                flat[base + 4] = t.y;
                flat[base + 5] = t.z;
                flat[base + 6] = t.t;
                flat[base + 7] = t.alpha;
                flat[base + 8] = t.mu;
                flat[base + 9] = t.sigma;
            }
        }
        let bytes = (count as usize * TENSOR_VOLUME_STRIDE_FLOATS as usize * 4) as wgpu::BufferAddress;
        queue.write_buffer(&self.nodes_buf, 0, bytemuck::cast_slice(&flat[..count as usize * 10]));
        let _ = bytes;
        true
    }

    /// GPU kNN filter; returns hit count written into `out` (caller stack buffer).
    pub fn tensor_search_into(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        query: &Tensor10D,
        max_distance: f32,
        out: &mut [usize],
    ) -> usize {
        let node_count = global_resident_substrate().node_count();
        if node_count == 0 || out.is_empty() || !self.upload_nodes(queue, node_count) {
            return 0;
        }

        queue.write_buffer(&self.query_buf, 0, bytemuck::bytes_of(query));
        let params = VolumeGpuParams {
            node_count,
            max_distance,
            stride_floats: TENSOR_VOLUME_STRIDE_FLOATS,
            max_hits: MAX_KNN_HITS as u32,
        };
        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&params));
        queue.write_buffer(&self.count_buf, 0, &[0u8; 4]);

        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("TensorVolumeBindGroup"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.query_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.nodes_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.hits_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.count_buf.as_entire_binding(),
                },
            ],
        });

        let wg = (node_count + 63) / 64;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("TensorVolumeEncoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("TensorVolumePass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(wg, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&self.hits_buf, 0, &self.staging_hits, 0, (MAX_KNN_HITS * 4) as u64);
        encoder.copy_buffer_to_buffer(&self.count_buf, 0, &self.staging_count, 0, 4);
        queue.submit(Some(encoder.finish()));

        device.poll(wgpu::Maintain::Wait);
        let count_slice = self.staging_count.slice(..4);
        let (tx, rx) = std::sync::mpsc::channel();
        count_slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::Maintain::Wait);
        if rx.recv().ok().and_then(|r| r.ok()).is_none() {
            return 0;
        }
        let count_data = count_slice.get_mapped_range();
        let total = u32::from_le_bytes(count_data[..4].try_into().unwrap_or([0; 4])) as usize;
        drop(count_data);
        self.staging_count.unmap();

        let hits_slice = self.staging_hits.slice(..(MAX_KNN_HITS * 4) as u64);
        let (tx2, rx2) = std::sync::mpsc::channel();
        hits_slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx2.send(r);
        });
        device.poll(wgpu::Maintain::Wait);
        if rx2.recv().ok().and_then(|r| r.ok()).is_none() {
            return 0;
        }
        let hits_data = hits_slice.get_mapped_range();
        let indices: &[u32] = bytemuck::cast_slice(&hits_data);
        let n = total.min(out.len()).min(indices.len());
        for i in 0..n {
            out[i] = indices[i] as usize;
        }
        drop(hits_data);
        self.staging_hits.unmap();
        n
    }
}

#[cfg(not(target_arch = "wasm32"))]
static VOLUME_GPU: std::sync::OnceLock<Option<TensorVolumeGpu>> = std::sync::OnceLock::new();

/// Try GPU tensor search; returns None when GPU path unavailable (caller uses SIMD).
#[cfg(not(target_arch = "wasm32"))]
pub fn try_gpu_tensor_search_into(
    query: &Tensor10D,
    max_distance: f32,
    out: &mut [usize],
) -> Option<usize> {
    let gpu_ctx = crate::gpu_context::shared_gpu();
    let vol = VOLUME_GPU.get_or_init(|| TensorVolumeGpu::try_new(&gpu_ctx.device));
    let vol = vol.as_ref()?;
    let n = vol.tensor_search_into(
        &gpu_ctx.device,
        crate::gpu_context::shared_gpu()
            .queue_for_universe(crate::gpu_context::ComputeUniverse::Tensor10D),
        query,
        max_distance,
        out,
    );
    if n > 0 {
        Some(n)
    } else {
        None
    }
}

#[cfg(target_arch = "wasm32")]
pub fn try_gpu_tensor_search_into(
    _query: &Tensor10D,
    _max_distance: f32,
    _out: &mut [usize],
) -> Option<usize> {
    None
}