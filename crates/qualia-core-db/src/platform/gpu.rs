//! GPU integration for calculus modality.
//!
//! Provides cross-platform GPU compute shader execution for numerical integration
//! and differential equation solving via portable `wgpu` (Vulkan / DX12 / Metal /
//! WebGPU). The engine is **vendor-neutral**: there is no CUDA/cuFile path. The
//! former NVIDIA GPUDirect-Storage bridge was removed; heterogeneous dispatch,
//! mixed-precision policy, and the vendor-neutral storage path (mmap + staging
//! upload) live in [`super::hetero_dispatch`].
//!
//! ## Architecture
//!
//! - **WebGPU (`wgpu`)**: portable compute on every backend; the single GPU path.
//! - **Storage**: NVMe → OS page cache (`mmap`, `super::host`) → GPU staging upload.
//!   (NVIDIA GDS's true NVMe→VRAM DMA was deliberately declined — vendor-lock off
//!   the affordability critical path; see `hetero_dispatch.rs`.)
//! - **State Tracking**: GPU results packed into Quin metadata field.
#![cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]

use crate::NQuin;
use std::io::Seek;
use std::path::Path;

/// The platform GPU integrator — uniformly the portable `wgpu` integrator on every
/// backend. (The former CUDA/cuFile `CudaIntegrator` alias was removed for vendor
/// neutrality; heterogeneous routing/fallback now lives in `super::hetero_dispatch`.)
pub use WebGpuIntegrator as PlatformGpuIntegrator;

// ─── Errors ─────────────────────────────────────────────────────────────────────

pub use etalon_wgsl_forge::numerical::compute_passes::calculus::{GpuError, WebGpuIntegrator};

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

    /// Executes composite Simpson 3/8 quadrature on the GPU.
    fn integrate_simpson_38_gpu(
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

    fn integrate_simpson_38_gpu(
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

        // Dispatch the composite Simpson 3/8 compute shader.
        let handle = tokio::runtime::Handle::try_current()
            .map_err(|e| GpuError::WebGPUUnavailable(format!("Tokio handle failed: {e}")))?;
        handle.block_on(self.execute_simpson_38_compute(&buffer, step_size))
    }

    fn available_vram(&self) -> u64 {
        // Use max_buffer_size from device limits as a practical proxy for available VRAM.
        // On discrete GPUs this cap is typically set to the full VRAM size; on integrated
        // GPUs it reflects the shared system memory window.
        self.max_buffer_size()
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
pub fn create_gpu_job_quin(job_id: u64, opcode: u8, file_offset: u64, step_size: f32) -> NQuin {
    let mut quin = NQuin::default();
    quin.subject = job_id;
    quin.predicate = (opcode as u64) | (q_hash("calculus:gpu") << 8);
    quin.object = file_offset;
    quin.context = f32::to_bits(step_size) as u64;
    quin.metadata = 0; // Will be filled with result
    quin.parity = 0; // Will be computed
    quin
}

// ─── Helper Functions ───────────────────────────────────────────────────────────

/// Compile-time hashing function (reused from lib.rs) — 60-bit identity, in
/// lockstep with `crate::q_hash` so the "calculus:gpu" predicate path joins.
const fn q_hash(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        hash = hash ^ (bytes[i] as u64);
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1;
    }
    hash & 0x0FFF_FFFF_FFFF_FFFF
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
            12345, 0x50, // OP_SIMPSONS_INTEGRATION
            4096, 0.001,
        );

        assert_eq!(quin.subject, 12345);
        assert_eq!(quin.object, 4096);
        assert_eq!(f32::from_bits(quin.context as u32), 0.001);
    }

    #[cfg(feature = "wgsl-forge")]
    #[test]
    fn calculus_shader_is_portable_and_semantically_named() {
        let source = etalon_wgsl_forge::numerical::compute_passes::calculus::CALCULUS_WGSL;
        crate::wgsl_forge::validate_wgsl(source).expect("calculus WGSL must Naga-validate");
        assert!(!source.contains("array<f64>"));
        assert!(!source.contains(": f64"));
        assert!(!source.contains("rk4_step"));
        assert!(source.contains("simpson_38_integration"));
    }
}
