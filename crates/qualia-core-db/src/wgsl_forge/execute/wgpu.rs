use std::sync::mpsc;
use std::time::Instant;

use super::compute::QualiaCompute;
use super::memory::{BufferView, MemoryTopology, QualiaSlabAllocator};
use crate::wgsl_forge::{AdapterConstraints, AdapterIdentity, ForgeError, Schedule};

pub struct WgpuComputeContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: AdapterIdentity,
    pub constraints: AdapterConstraints,
    pub allocator: QualiaSlabAllocator,
    pub slab: wgpu::Buffer,
    pub timestamp_supported: bool,
    pub timestamp_period_ns: f32,
    timestamp_resources: Option<TimestampResources>,
}

impl WgpuComputeContext {
    pub fn new(capacity_bytes: usize) -> Result<Self, ForgeError> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .map_err(|error| ForgeError::GpuUnavailable(error.to_string()))?;
        let info = adapter.get_info();
        let available_features = adapter.features();
        let required_features = available_features & wgpu::Features::TIMESTAMP_QUERY;
        let timestamp_supported = required_features.contains(wgpu::Features::TIMESTAMP_QUERY);
        let limits = adapter.limits();
        let constraints = AdapterConstraints::from_wgpu_limits(&limits);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_features,
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }))
        .map_err(|error| ForgeError::GpuUnavailable(error.to_string()))?;
        let timestamp_period_ns = if timestamp_supported {
            queue.get_timestamp_period()
        } else {
            0.0
        };

        // Determine topology (unified vs discrete)
        let topology = if info.device_type == wgpu::DeviceType::IntegratedGpu || info.device_type == wgpu::DeviceType::Cpu {
            MemoryTopology::Unified { zero_copy: true }
        } else {
            MemoryTopology::Discrete { staging_required: true }
        };

        let allocator = QualiaSlabAllocator::new(topology, capacity_bytes);
        
        let slab = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forge-slab"),
            size: capacity_bytes as u64,
            usage: wgpu::BufferUsages::STORAGE 
                | wgpu::BufferUsages::UNIFORM 
                | wgpu::BufferUsages::COPY_DST 
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let timestamp_resources = timestamp_supported.then(|| TimestampResources::new(&device));

        Ok(Self {
            device,
            queue,
            adapter: AdapterIdentity {
                name: info.name,
                vendor: info.vendor,
                device: info.device,
                device_type: format!("{:?}", info.device_type),
                backend: format!("{:?}", info.backend),
                driver: info.driver,
                driver_info: info.driver_info,
            },
            constraints,
            allocator,
            slab,
            timestamp_supported,
            timestamp_period_ns,
            timestamp_resources,
        })
    }

    pub fn allocate_and_write(&mut self, data: &[u8], binding: u32, group: u32) -> Result<BufferView, ForgeError> {
        let view = self.allocator.allocate_transient(data.len(), binding, group)?;
        if !data.is_empty() {
            self.queue.write_buffer(&self.slab, view.offset as wgpu::BufferAddress, data);
        }
        Ok(view)
    }

    pub fn allocate_transient(&mut self, size_bytes: usize, binding: u32, group: u32) -> Result<BufferView, ForgeError> {
        self.allocator.allocate_transient(size_bytes, binding, group)
    }

    pub fn advance_read_head(&mut self, offset: usize) {
        self.allocator.advance_read_head(offset);
    }
    
    pub fn clear_transient_allocations(&mut self) {
        self.allocator.clear();
    }
    
    pub fn read_buffer_f32(&self, view: &BufferView) -> Result<Vec<f32>, ForgeError> {
        let size = view.length_bytes as u64;
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forge-output-staging"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("forge-output-copy"),
        });
        encoder.copy_buffer_to_buffer(&self.slab, view.offset as u64, &staging, 0, size);
        self.queue.submit(Some(encoder.finish()));
        let bytes = map_read(&self.device, &staging)?;
        
        let elements = view.length_bytes / size_of::<f32>();
        let output = bytemuck::cast_slice::<u8, f32>(&bytes)[..elements].to_vec();
        drop(bytes);
        staging.unmap();
        Ok(output)
    }
}

pub struct WgpuPipeline<'a> {
    context: &'a WgpuComputeContext,
    pipeline: wgpu::ComputePipeline,
}

impl<'a> WgpuPipeline<'a> {
    pub fn compile(context: &'a WgpuComputeContext, source: &str, entry_point: &str) -> Result<Self, ForgeError> {
        let error_scope = context.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = context.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qualia-wgsl-forge"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = context.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("qualia-wgsl-forge-pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some(entry_point),
            compilation_options: Default::default(),
            cache: None,
        });
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            return Err(ForgeError::GpuValidation(error.to_string()));
        }
        Ok(Self { context, pipeline })
    }
}

impl<'a> QualiaCompute for WgpuPipeline<'a> {
    fn dispatch(
        &self,
        buffers: &[BufferView],
        schedule: &Schedule,
        element_count: usize,
    ) -> Result<u64, ForgeError> {
        let mut entries = Vec::with_capacity(buffers.len());
        for view in buffers {
            let size = wgpu::BufferSize::new(view.length_bytes as u64);
            entries.push(wgpu::BindGroupEntry {
                binding: view.binding,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &self.context.slab,
                    offset: view.offset as wgpu::BufferAddress,
                    size,
                }),
            });
        }
        
        let bind_group = self.context.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("forge-affine-bind-group"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let dispatch_x = schedule.dispatch_workgroups(element_count);

        let started = Instant::now();
        let mut encoder = self.context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("forge-dispatch"),
        });
        {
            let timestamp_writes = self.context.timestamp_resources.as_ref().map(|resources| wgpu::ComputePassTimestampWrites {
                query_set: &resources.query_set,
                beginning_of_pass_write_index: Some(0),
                end_of_pass_write_index: Some(1),
            });
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("forge-compute-pass"),
                timestamp_writes,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(dispatch_x, 1, 1);
        }
        
        if let Some(resources) = &self.context.timestamp_resources {
            encoder.resolve_query_set(&resources.query_set, 0..2, &resources.resolve, 0);
            encoder.copy_buffer_to_buffer(&resources.resolve, 0, &resources.staging, 0, 16);
        }
        self.context.queue.submit(Some(encoder.finish()));

        if let Some(resources) = &self.context.timestamp_resources {
            let bytes = map_read(&self.context.device, &resources.staging)?;
            let ticks: &[u64] = bytemuck::cast_slice(&bytes);
            let elapsed = ticks
                .get(1)
                .copied()
                .unwrap_or(0)
                .saturating_sub(ticks.first().copied().unwrap_or(0));
            drop(bytes);
            resources.staging.unmap();
            Ok((elapsed as f64 * self.context.timestamp_period_ns as f64) as u64)
        } else {
            let _ = self.context.device.poll(wgpu::PollType::wait_indefinitely());
            Ok(started.elapsed().as_nanos().min(u64::MAX as u128) as u64)
        }
    }
}

struct TimestampResources {
    query_set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    staging: wgpu::Buffer,
}

impl TimestampResources {
    fn new(device: &wgpu::Device) -> Self {
        Self {
            query_set: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("forge-timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: 2,
            }),
            resolve: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("forge-timestamp-resolve"),
                size: 16,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            staging: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("forge-timestamp-staging"),
                size: 16,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
        }
    }
}

fn map_read(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Result<wgpu::BufferView, ForgeError> {
    let slice = buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    receiver
        .recv()
        .map_err(|error| ForgeError::GpuValidation(error.to_string()))?
        .map_err(|error| ForgeError::GpuValidation(error.to_string()))?;
    Ok(slice.get_mapped_range())
}
