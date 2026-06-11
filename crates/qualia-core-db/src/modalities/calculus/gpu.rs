//! GPU integration for calculus modality.
//!
//! Provides cross-platform GPU compute shader execution for numerical integration
//! and differential equation solving. Integrates with DirectStorage (Windows),
//! GPUDirect (Linux, optional feature), and WebGPU fallback.
//!
//! ## Architecture
//!
//! - **DirectStorage**: NVMe → GPU VRAM DMA bypass (Windows)
//! - **GPUDirect**: NVMe → GPU VRAM DMA bypass (Linux, optional feature `cuda_gds`)
//! - **WebGPU**: CPU RAM → GPU VRAM fallback (cross-platform)
//! - **State Tracking**: GPU results packed into Quin metadata field
//!
//! ## Feature Flags
//!
//! - `cuda_gds`: Enable CUDA GPUDirect Storage support (Linux only, requires NVIDIA drivers)

use crate::NQuin;
use std::path::Path;
use std::io::Seek;
use wgpu::util::DeviceExt;

// Conditionally compile CUDA bridge
#[cfg(all(target_os = "linux", feature = "cuda_gds"))]
mod cuda_bridge;

#[cfg(all(target_os = "linux", feature = "cuda_gds"))]
pub use cuda_bridge::CudaIntegrator as PlatformGpuIntegrator;

#[cfg(not(all(target_os = "linux", feature = "cuda_gds")))]
pub use WebGpuIntegrator as PlatformGpuIntegrator;

// ─── Errors ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum GpuError {
    DirectStorageUnavailable(String),
    GpuDirectUnavailable(String),
    WebGPUUnavailable(String),
    ShaderCompilationFailed(String),
    BufferAllocationFailed(String),
    DispatchFailed(String),
    ReadbackFailed(String),
    InvalidOffset { offset: u64, required: u64 },
}

// ─── GPU Integration Trait ─────────────────────────────────────────────────────

/// Platform-agnostic GPU integration interface.
///
/// Abstracts over DirectStorage, GPUDirect, and WebGPU to provide a unified
/// API for GPU-accelerated calculus operations.
pub trait GpuIntegrator: Send {
    /// Executes Simpson's rule integration on the GPU.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the data file (NVMe storage)
    /// * `offset` - Byte offset in file (must be 4096-byte aligned)
    /// * `size` - Number of bytes to process (must be 4096-byte aligned)
    /// * `step_size` - Integration step size (f32 for Quin packing)
    ///
    /// # Returns
    ///
    /// The integrated value as f64, ready to pack into Quin metadata.
    fn integrate_simpsons_gpu(
        &mut self,
        file_path: &Path,
        offset: u64,
        size: u64,
        step_size: f32,
    ) -> Result<f64, GpuError>;
    
    /// Executes Runge-Kutta 4th order ODE step on the GPU.
    fn rk4_step_gpu(
        &mut self,
        file_path: &Path,
        offset: u64,
        size: u64,
        step_size: f32,
    ) -> Result<f64, GpuError>;
    
    /// Returns available VRAM in bytes.
    fn available_vram(&self) -> u64;
}

// ─── WebGPU Fallback Implementation ─────────────────────────────────────────────

/// WebGPU-based GPU integrator (cross-platform fallback).
///
/// Uses wgpu to execute compute shaders when DirectStorage or GPUDirect
/// are unavailable. Data flows: NVMe → CPU RAM → GPU VRAM.
pub struct WebGpuIntegrator {
    device: wgpu::Device,
    queue: wgpu::Queue,
    compute_pipeline: wgpu::ComputePipeline,
    rk4_pipeline: wgpu::ComputePipeline,
}

impl WebGpuIntegrator {
    /// Creates a new WebGPU integrator.
    ///
    /// Initializes wgpu device, queue, and compiles the calculus compute shader.
    pub async fn new() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::default();
        
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .ok_or(GpuError::WebGPUUnavailable("No adapter found".to_string()))?;
        
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .map_err(|e| GpuError::WebGPUUnavailable(format!("Device request failed: {e}")))?;
        
        // Load pre-compiled compute shader (AOT compilation)
        let shader_src = include_str!("../../shaders/calculus.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Calculus Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });
        
        let compute_pipeline = device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Calculus Pipeline"),
                layout: None,
                module: &shader,
                entry_point: "simpsons_integration",
            });

        let rk4_pipeline = device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("RK4 Pipeline"),
                layout: None,
                module: &shader,
                entry_point: "rk4_step",
            });

        Ok(Self {
            device,
            queue,
            compute_pipeline,
            rk4_pipeline,
        })
    }
    
    /// Executes a compute shader with the given input data.
    async fn execute_compute(
        &self,
        input_data: &[u8],
        step_size: f32,
    ) -> Result<f64, GpuError> {
        // Create storage buffer for input data
        let input_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Input Buffer"),
            contents: input_data,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        
        // Create workgroup reduction buffer (one f64 per workgroup)
        // For 5000 elements with 64-thread workgroups: ceil(5000/64) = 79 workgroups
        let num_workgroups = ((input_data.len() / 8) + 63) / 64;  // f64 bytes / 64 threads
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Workgroup Reduction Buffer"),
            size: (num_workgroups * 8) as u64,  // f64 per workgroup
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        // Create uniform buffer for step size and total element count
        #[repr(C)]
        #[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
        struct Uniforms {
            step_size: f32,
            total_elements: u32,
        }
        
        let uniforms = Uniforms {
            step_size,
            total_elements: (input_data.len() / 8) as u32,  // Number of f64 values
        };
        
        let step_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniforms Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        // Create bind group
        let bind_group_layout = self.compute_pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Calculus Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: step_buffer.as_entire_binding(),
                },
            ],
        });
        
        // Create command encoder
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Calculus Encoder"),
        });
        
        // Dispatch compute shader
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Calculus Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);
        }
        
        // Copy output to staging buffer for readback
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: (num_workgroups * 8) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        
        encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, (num_workgroups * 8) as u64);
        
        // Submit commands
        self.queue.submit(Some(encoder.finish()));
        
        // Read back result
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures_channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        
        self.queue.submit(None);
        
        let _mapping_result = receiver.await.unwrap();
        
        // Get mapped slice
        let result_data = buffer_slice.get_mapped_range();
        
        // Read workgroup results and sum using Kahan summation for precision
        let workgroup_results: &[f64] = bytemuck::cast_slice(&*result_data);
        let mut sum = 0.0f64;
        let mut compensation = 0.0f64;
        
        for &partial in workgroup_results {
            let y = partial - compensation;
            let t = sum + y;
            compensation = (t - sum) - y;
            sum = t;
        }
        
        Ok(sum)
    }

    /// RK4 variant of execute_compute, using the rk4_step pipeline.
    async fn execute_rk4_compute(
        &self,
        input_data: &[u8],
        step_size: f32,
    ) -> Result<f64, GpuError> {
        let input_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("RK4 Input Buffer"),
            contents: input_data,
            usage:    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let num_workgroups = ((input_data.len() / 8) + 63) / 64;
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label:              Some("RK4 Reduction Buffer"),
            size:               (num_workgroups * 8) as u64,
            usage:              wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
        struct Uniforms { step_size: f32, total_elements: u32 }

        let uniforms = Uniforms {
            step_size,
            total_elements: (input_data.len() / 8) as u32,
        };
        let step_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("RK4 Uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage:    wgpu::BufferUsages::UNIFORM,
        });

        let bgl = self.rk4_pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("RK4 Bind Group"),
            layout:  &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: output_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: step_buffer.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("RK4 Encoder") },
        );
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label:            Some("RK4 Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.rk4_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
        }

        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label:              Some("RK4 Staging"),
            size:               (num_workgroups * 8) as u64,
            usage:              wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging, 0, (num_workgroups * 8) as u64);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = futures_channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        self.queue.submit(None);
        let _ = rx.await.unwrap();

        let mapped = slice.get_mapped_range();
        let partials: &[f64] = bytemuck::cast_slice(&*mapped);

        // Kahan summation for numerical stability
        let mut sum = 0.0f64;
        let mut comp = 0.0f64;
        for &p in partials {
            let y = p - comp;
            let t = sum + y;
            comp = (t - sum) - y;
            sum = t;
        }
        Ok(sum)
    }
}

#[async_trait::async_trait]
impl GpuIntegrator for WebGpuIntegrator {
    fn integrate_simpsons_gpu(
        &mut self,
        file_path: &Path,
        offset: u64,
        size: u64,
        step_size: f32,
    ) -> Result<f64, GpuError> {
        // Read data from file (CPU RAM path - fallback)
        let mut file = std::fs::File::open(file_path)
            .map_err(|e| GpuError::WebGPUUnavailable(format!("File open failed: {e}")))?;
        
        use std::io::Read;
        file.seek(std::io::SeekFrom::Start(offset))
            .map_err(|e| GpuError::WebGPUUnavailable(format!("Seek failed: {e}")))?;
        
        let mut buffer = vec![0u8; size as usize];
        file.read_exact(&mut buffer)
            .map_err(|e| GpuError::WebGPUUnavailable(format!("Read failed: {e}")))?;
        
        // Execute on GPU
        // Note: This is a blocking call in an async context - in production,
        // we'd use a thread pool or async executor
        let result = self.execute_compute(&buffer, step_size);
        
        // For now, we'll block on the async result using the current runtime
        // In production, this should be properly integrated with the async runtime
        let handle = tokio::runtime::Handle::try_current()
            .map_err(|e| GpuError::WebGPUUnavailable(format!("Tokio handle failed: {e}")))?;
        handle.block_on(result)
    }
    
    fn rk4_step_gpu(
        &mut self,
        file_path: &Path,
        offset: u64,
        size: u64,
        step_size: f32,
    ) -> Result<f64, GpuError> {
        // Read data from file (CPU RAM path)
        let mut file = std::fs::File::open(file_path)
            .map_err(|e| GpuError::WebGPUUnavailable(format!("File open failed: {e}")))?;

        use std::io::Read;
        file.seek(std::io::SeekFrom::Start(offset))
            .map_err(|e| GpuError::WebGPUUnavailable(format!("Seek failed: {e}")))?;

        let mut buffer = vec![0u8; size as usize];
        file.read_exact(&mut buffer)
            .map_err(|e| GpuError::WebGPUUnavailable(format!("Read failed: {e}")))?;

        // Dispatch the rk4_step compute shader
        let handle = tokio::runtime::Handle::try_current()
            .map_err(|e| GpuError::WebGPUUnavailable(format!("Tokio handle failed: {e}")))?;
        handle.block_on(self.execute_rk4_compute(&buffer, step_size))
    }

    fn available_vram(&self) -> u64 {
        // Use max_buffer_size from device limits as a practical proxy for available VRAM.
        // On discrete GPUs this cap is typically set to the full VRAM size; on integrated
        // GPUs it reflects the shared system memory window.
        self.device.limits().max_buffer_size
    }
}

// ─── GPU State Tracking ─────────────────────────────────────────────────────────

/// Packs GPU computation result into Quin metadata field.
///
/// When the GPU finishes processing, the scalar result is written back
/// to the host and packed into the Quin's metadata field for SLG VM resumption.
pub fn pack_gpu_result_into_quin(quin: &mut NQuin, result: f64) {
    quin.metadata = f64::to_bits(result);
}

/// Extracts GPU computation result from Quin metadata field.
pub fn extract_gpu_result_from_quin(quin: &NQuin) -> f64 {
    f64::from_bits(quin.metadata)
}

/// Creates a suspended Quin for GPU computation.
///
/// Packs the computation parameters into the Quin fields so the SLG VM
/// can track the in-flight GPU operation.
pub fn create_gpu_job_quin(
    job_id: u64,
    opcode: u8,
    file_offset: u64,
    step_size: f32,
) -> NQuin {
    let mut quin = NQuin::default();
    quin.subject = job_id;
    quin.predicate = (opcode as u64) | (q_hash("calculus:gpu") << 8);
    quin.object = file_offset;
    quin.context = f32::to_bits(step_size) as u64;
    quin.metadata = 0;  // Will be filled with result
    quin.parity = 0;  // Will be computed
    quin
}

// ─── Helper Functions ───────────────────────────────────────────────────────────

/// Compile-time hashing function (reused from lib.rs).
const fn q_hash(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        hash = hash ^ (bytes[i] as u64);
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1;
    }
    hash
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gpu_result_packing() {
        let result = 42.5f64;
        let mut quin = NQuin::default();
        
        pack_gpu_result_into_quin(&mut quin, result);
        
        let extracted = extract_gpu_result_from_quin(&quin);
        assert_eq!(extracted, result);
    }
    
    #[test]
    fn test_gpu_job_quin_creation() {
        let quin = create_gpu_job_quin(
            12345,
            0x50,  // OP_SIMPSONS_INTEGRATION
            4096,
            0.001,
        );
        
        assert_eq!(quin.subject, 12345);
        assert_eq!(quin.object, 4096);
        assert_eq!(f32::from_bits(quin.context as u32), 0.001);
    }
}
