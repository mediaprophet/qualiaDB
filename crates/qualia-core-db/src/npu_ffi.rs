//! GPU/NPU Sieve
//! Uses `wgpu` to execute cross-platform compute shaders (Vulkan/DirectML/Metal/WebGPU)
//! that filter the 5th Vector of the QualiaQuin parallel arrays.

#[cfg(not(target_arch = "wasm32"))]
pub mod gpu_sieve {
    use wgpu::util::DeviceExt;
    use crate::QualiaQuin;

    #[repr(C)]
    #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
    pub struct FilterMask {
        pub target_mask: u32,
    }

    pub struct SieveOrchestrator {
        device: wgpu::Device,
        queue: wgpu::Queue,
        compute_pipeline: wgpu::ComputePipeline,
    }

    impl SieveOrchestrator {
        pub async fn new() -> Option<Self> {
            let instance = wgpu::Instance::default();
            let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await?;
            let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default(), None).await.ok()?;

            let shader_src = include_str!("shaders/sieve.wgsl");
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Sieve Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_src.into()),
            });

            let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Sieve Pipeline"),
                layout: None,
                module: &shader,
                entry_point: "main",
            });

            Some(Self {
                device,
                queue,
                compute_pipeline,
            })
        }

        /// Executes the PEXT filter across the GPU and returns an array of matched indices (1 or 0)
        pub async fn execute_sieve(&self, quins: &[QualiaQuin], mask: u32) -> Option<Vec<u32>> {
            let filter = FilterMask { target_mask: mask };
            
            // 1. Storage Buffer for Quins
            let quin_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Quin Buffer"),
                contents: bytemuck::cast_slice(quins),
                usage: wgpu::BufferUsages::STORAGE,
            });

            // 2. Storage Buffer for Results
            let result_size = (quins.len() * std::mem::size_of::<u32>()) as wgpu::BufferAddress;
            let result_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Result Buffer"),
                size: result_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });

            // 3. Uniform Buffer for the Filter Mask
            let filter_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Filter Buffer"),
                contents: bytemuck::cast_slice(&[filter]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            let bind_group_layout = self.compute_pipeline.get_bind_group_layout(0);
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Sieve Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: quin_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: result_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: filter_buffer.as_entire_binding() },
                ],
            });

            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
                cpass.set_pipeline(&self.compute_pipeline);
                cpass.set_bind_group(0, &bind_group, &[]);
                
                // Dispatch logic: 64 threads per workgroup
                let workgroups = ((quins.len() as f32) / 64.0).ceil() as u32;
                cpass.dispatch_workgroups(if workgroups == 0 { 1 } else { workgroups }, 1, 1);
            }

            // Copy results back to a staging buffer
            let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Staging Buffer"),
                size: result_size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            encoder.copy_buffer_to_buffer(&result_buffer, 0, &staging_buffer, 0, result_size);

            self.queue.submit(Some(encoder.finish()));
            
            crate::telemetry::SIEVE_OPS_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            // Await completion
            let buffer_slice = staging_buffer.slice(..);
            let (sender, receiver) = futures_channel::oneshot::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
            
            self.device.poll(wgpu::Maintain::Wait);
            
            if receiver.await.is_ok() {
                let data = buffer_slice.get_mapped_range();
                let result: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
                drop(data);
                staging_buffer.unmap();
                return Some(result);
            }

            None
        }
    }
}
