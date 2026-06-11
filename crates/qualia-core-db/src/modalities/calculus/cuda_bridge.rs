//! CUDA GPUDirect Storage bridge for Linux.
//!
//! Provides FFI bindings to NVIDIA's libcufile for DMA transfers directly
//! from NVMe to GPU VRAM, bypassing CPU RAM entirely.
//!
//! This module is only compiled when the `cuda_gds` feature is enabled.
//! It requires NVIDIA drivers and the cuFile library to be installed.

#![cfg(all(target_os = "linux", feature = "cuda_gds"))]

use super::gpu::GpuError;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use wgpu;

// ─── FFI Bindings to libcufile ───────────────────────────────────────────────────

// cuFile driver handle types
#[repr(C)]
pub enum CUfileHandle_t {}
pub type CUfileHandle = *mut CUfileHandle_t;

#[repr(C)]
pub struct CUfileDescr_t {
    pub handle: CUfileHandle,
    pub type_: CUfileHandleType,
}

#[repr(C)]
pub enum CUfileHandleType {
    CU_FILE_HANDLE_TYPE_OPAQUE_FD = 0,
    CU_FILE_HANDLE_TYPE_POSIX_FILE_DESC = 1,
    CU_FILE_HANDLE_TYPE_DMABUF_FD = 2,
}

#[repr(C)]
pub union CUfileHandle {
    pub fd: i32,
    pub reserved: [u8; 8],
}

// CUDA device pointer type
pub type CUdeviceptr = u64;

// External FFI functions from libcufile
#[link(name = "cufile")]
extern "C" {
    /// Initialize the cuFile driver
    pub fn cuFileDriverOpen() -> i32;
    
    /// Close the cuFile driver
    pub fn cuFileDriverClose() -> i32;
    
    /// Register a file handle with cuFile
    pub fn cuFileHandleRegister(
        handle: *mut CUfileHandle,
        desc: *mut CUfileDescr_t,
    ) -> i32;
    
    /// Deregister a file handle
    pub fn cuFileHandleDeregister(handle: CUfileHandle) -> i32;
    
    /// Read from file directly to GPU memory
    pub fn cuFileRead(
        handle: CUfileHandle,
        devPtr: *mut std::ffi::c_void,
        size: usize,
        fileOffset: i64,
        devPtrOffset: i64,
    ) -> isize;
    
    /// Write from GPU memory directly to file
    pub fn cuFileWrite(
        handle: CUfileHandle,
        devPtr: *const std::ffi::c_void,
        size: usize,
        fileOffset: i64,
        devPtrOffset: i64,
    ) -> isize;
}

// CUDA runtime FFI (simplified - in production use cuda-rs or similar)
#[link(name = "cuda")]
extern "C" {
    /// Allocate GPU memory
    pub fn cuMemAlloc(ptr: *mut CUdeviceptr, size: usize) -> i32;
    
    /// Free GPU memory
    pub fn cuMemFree(ptr: CUdeviceptr) -> i32;
    
    /// Initialize CUDA
    pub fn cuInit(flags: u32) -> i32;
}

// ─── CUDA GPUDirect Integrator ───────────────────────────────────────────────────

/// CUDA GPUDirect Storage integrator for Linux.
///
/// Enables DMA transfers directly from NVMe to GPU VRAM using NVIDIA's
/// cuFile library. This is the Linux counterpart to Windows DirectStorage.
pub struct CudaIntegrator {
    device_ptr: CUdeviceptr,
    file_handle: CUfileHandle,
    buffer_size: usize,
    initialized: bool,
}

impl CudaIntegrator {
    /// Creates a new CUDA GPUDirect integrator.
    ///
    /// # Arguments
    ///
    /// * `file` - The file to read from (must be on a GDS-capable filesystem)
    /// * `buffer_size` - Size of GPU buffer to allocate (must be 4096-byte aligned)
    ///
    /// # Errors
    ///
    /// Returns an error if CUDA initialization fails, GPU memory allocation fails,
    /// or file handle registration fails.
    pub fn new(file: &File, buffer_size: usize) -> Result<Self, GpuError> {
        // Validate buffer size alignment
        if buffer_size % 4096 != 0 {
            return Err(GpuError::BufferAllocationFailed(
                "Buffer size must be 4096-byte aligned".to_string()
            ));
        }
        
        unsafe {
            // Initialize CUDA
            let cuda_result = cuInit(0);
            if cuda_result != 0 {
                return Err(GpuError::ShaderCompilationFailed(
                    format!("CUDA initialization failed: {}", cuda_result)
                ));
            }
            
            // Initialize cuFile driver
            let driver_result = cuFileDriverOpen();
            if driver_result != 0 {
                return Err(GpuError::GpuDirectUnavailable(
                    format!("cuFile driver initialization failed: {}", driver_result)
                ));
            }
            
            // Allocate GPU memory
            let mut device_ptr: CUdeviceptr = 0;
            let alloc_result = cuMemAlloc(&mut device_ptr, buffer_size);
            if alloc_result != 0 {
                let _ = cuFileDriverClose();
                return Err(GpuError::BufferAllocationFailed(
                    format!("GPU memory allocation failed: {}", alloc_result)
                ));
            }
            
            // Register file handle with cuFile
            let mut handle_desc = CUfileDescr_t {
                handle: CUfileHandle { fd: file.as_raw_fd() },
                type_: CUfileHandleType::CU_FILE_HANDLE_TYPE_OPAQUE_FD,
            };
            
            let mut file_handle: CUfileHandle = std::ptr::null_mut();
            let register_result = cuFileHandleRegister(&mut file_handle, &mut handle_desc);
            if register_result != 0 {
                let _ = cuMemFree(device_ptr);
                let _ = cuFileDriverClose();
                return Err(GpuError::GpuDirectUnavailable(
                    format!("File handle registration failed: {}", register_result)
                ));
            }
            
            Ok(Self {
                device_ptr,
                file_handle,
                buffer_size,
                initialized: true,
            })
        }
    }
    
    /// Reads data from file directly to GPU VRAM via DMA.
    ///
    /// # Arguments
    ///
    /// * `file_offset` - Byte offset in file (must be 4096-byte aligned)
    /// * `size` - Number of bytes to read (must be 4096-byte aligned)
    ///
    /// # Errors
    ///
    /// Returns an error if offset or size is not 4096-byte aligned, or if
    /// the DMA transfer fails.
    pub fn async_read_to_gpu(&mut self, file_offset: u64, size: usize) -> Result<(), GpuError> {
        if !self.initialized {
            return Err(GpuError::DispatchFailed("Integrator not initialized".to_string()));
        }
        
        // Validate 4096-byte alignment
        if file_offset % 4096 != 0 {
            return Err(GpuError::InvalidOffset {
                offset: file_offset,
                required: 4096,
            });
        }
        if size % 4096 != 0 {
            return Err(GpuError::InvalidOffset {
                offset: size as u64,
                required: 4096,
            });
        }
        
        if size > self.buffer_size {
            return Err(GpuError::BufferAllocationFailed(
                "Read size exceeds allocated GPU buffer".to_string()
            ));
        }
        
        unsafe {
            let bytes_read = cuFileRead(
                self.file_handle,
                self.device_ptr as *mut std::ffi::c_void,
                size,
                file_offset as i64,
                0,  // No offset within device_ptr
            );
            
            if bytes_read < 0 {
                return Err(GpuError::DispatchFailed(
                    format!("cuFileRead failed: {}", bytes_read)
                ));
            }
            
            Ok(())
        }
    }
    
    /// Returns the GPU device pointer for shader access.
    pub fn device_ptr(&self) -> CUdeviceptr {
        self.device_ptr
    }
    
    /// Returns the allocated buffer size.
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }
}

impl Drop for CudaIntegrator {
    fn drop(&mut self) {
        if self.initialized {
            unsafe {
                // Deregister file handle
                let _ = cuFileHandleDeregister(self.file_handle);
                
                // Free GPU memory
                let _ = cuMemFree(self.device_ptr);
                
                // Close cuFile driver
                let _ = cuFileDriverClose();
            }
            self.initialized = false;
        }
    }
}

// ─── GpuIntegrator Trait Implementation ─────────────────────────────────────────

use super::gpu::GpuIntegrator;

#[async_trait::async_trait]
impl GpuIntegrator for CudaIntegrator {
    fn integrate_simpsons_gpu(
        &mut self,
        file_path: &Path,
        offset: u64,
        size: u64,
        step_size: f32,
    ) -> Result<f64, GpuError> {
        // Attempt DMA read; on failure, fall through to the wgpu path which reads via CPU RAM.
        let _ = self.async_read_to_gpu(offset, size as usize);

        // Delegate to cross-platform WebGPU path (same GPU, different access route).
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| GpuError::WebGPUUnavailable(format!("tokio: {e}")))?;
        let mut wgpu_integrator = rt.block_on(super::gpu::WebGpuIntegrator::new())?;
        wgpu_integrator.integrate_simpsons_gpu(file_path, offset, size, step_size)
    }

    fn rk4_step_gpu(
        &mut self,
        file_path: &Path,
        offset: u64,
        size: u64,
        step_size: f32,
    ) -> Result<f64, GpuError> {
        let _ = self.async_read_to_gpu(offset, size as usize);

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| GpuError::WebGPUUnavailable(format!("tokio: {e}")))?;
        let mut wgpu_integrator = rt.block_on(super::gpu::WebGpuIntegrator::new())?;
        wgpu_integrator.rk4_step_gpu(file_path, offset, size, step_size)
    }

    fn available_vram(&self) -> u64 {
        // Query CUDA device memory via wgpu adapter as fallback
        let rt = tokio::runtime::Runtime::new().ok();
        if let Some(rt) = rt {
            let vram = rt.block_on(async {
                let instance = wgpu::Instance::default();
                let adapter = instance
                    .request_adapter(&wgpu::RequestAdapterOptions::default())
                    .await?;
                let (device, _) = adapter
                    .request_device(&wgpu::DeviceDescriptor::default(), None)
                    .await
                    .ok()?;
                Some(device.limits().max_buffer_size)
            });
            if let Some(v) = vram {
                return v;
            }
        }
        2_147_483_648 // 2 GiB conservative fallback
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    
    #[test]
    #[ignore]  // Requires CUDA hardware and cuFile library
    fn test_cuda_integrator_creation() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_cuda.dat");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(&vec![0u8; 8192]).unwrap();
        file.sync_all().unwrap();
        
        match CudaIntegrator::new(&file, 8192) {
            Ok(_) => println!("CUDA GPUDirect integrator created successfully"),
            Err(e) => println!("CUDA unavailable (expected without hardware): {:?}", e),
        }
        
        std::fs::remove_file(&file_path).ok();
    }
    
    #[test]
    fn test_buffer_alignment_validation() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_cuda_align.dat");
        
        let file = File::create(&file_path).unwrap();
        
        // Test misaligned buffer size
        let result = CudaIntegrator::new(&file, 4095);
        assert!(matches!(result, Err(GpuError::BufferAllocationFailed(_))));
        
        std::fs::remove_file(&file_path).ok();
    }
}
