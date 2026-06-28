//! Native CUDA execution backend for the WGSL Forge differential oracle.
//!
//! This bridges PTX modules emitted by [`crate::wgsl_forge::emit::ptx`] onto the
//! NVIDIA driver API via `cudarc` 0.19. It implements the same stateless
//! [`QualiaCompute`] contract as the wgpu backend: a single persistent device
//! slab is pre-allocated, and dispatches receive lightweight byte offsets
//! ([`BufferView`]) rather than allocating in the hot loop.
//!
//! cudarc 0.19 (driver redesign): allocation, copies, module loading and launch
//! all hang off a [`CudaContext`]/[`CudaStream`] pair. With the default
//! `fallback-dynamic-loading` feature the crate compiles without the CUDA
//! toolkit present and resolves libraries at runtime, so an absent toolkit
//! degrades to a runtime [`ForgeError::GpuUnavailable`] rather than a build
//! failure.

#[cfg(feature = "cuda")]
use std::sync::Arc;

#[cfg(feature = "cuda")]
use cudarc::driver::{
    CudaContext, CudaFunction, CudaModule, CudaSlice, CudaStream, DevicePtr, DeviceRepr,
    LaunchConfig, PushKernelArg,
};
#[cfg(feature = "cuda")]
use cudarc::nvrtc::Ptx;

#[cfg(feature = "cuda")]
use super::compute::QualiaCompute;
#[cfg(feature = "cuda")]
use super::memory::{BindingUsage, BufferView, MemoryTopology, QualiaSlabAllocator};
#[cfg(feature = "cuda")]
use crate::wgsl_forge::{AdapterConstraints, AdapterIdentity, ForgeError, Schedule};

/// 16-byte affine uniform block passed to the kernel by value.
///
/// The PTX emitter declares the uniform as `.param .align 4 .b8 params[16]`, so
/// we forward the raw 16 bytes read back from the slab verbatim. `DeviceRepr`'s
/// default `as_kernel_param` copies these bytes straight into the kernel's
/// parameter space.
#[cfg(feature = "cuda")]
#[repr(C)]
#[derive(Clone, Copy)]
struct AffineParamsRaw {
    bytes: [u8; 16],
}

#[cfg(feature = "cuda")]
unsafe impl DeviceRepr for AffineParamsRaw {}

#[cfg(feature = "cuda")]
pub struct CudaComputeContext {
    pub ctx: Arc<CudaContext>,
    pub stream: Arc<CudaStream>,
    pub adapter: AdapterIdentity,
    pub constraints: AdapterConstraints,
    pub allocator: QualiaSlabAllocator,
    pub slab: CudaSlice<u8>,
}

#[cfg(feature = "cuda")]
impl CudaComputeContext {
    pub fn new(capacity_bytes: usize) -> Result<Self, ForgeError> {
        let ctx = CudaContext::new(0)
            .map_err(|e| ForgeError::GpuUnavailable(format!("CUDA init failed: {:?}", e)))?;
        let stream = ctx.default_stream();

        let adapter = AdapterIdentity {
            name: "CUDA Device".to_string(),
            vendor: 4318, // NVIDIA
            device: 0,
            device_type: "DiscreteGpu".to_string(),
            backend: "CUDA".to_string(),
            driver: "cudarc".to_string(),
            driver_info: "0.19".to_string(),
        };

        // Conservative, honest constraints. cudarc abstracts the wgpu limit
        // surface, so we declare the NVIDIA block ceiling and warp presence and
        // leave cooperative-matrix (tensor-core) detection to the capability
        // probe rather than assuming it here.
        let constraints = AdapterConstraints {
            max_workgroup_size_x: 1024,
            max_invocations_per_workgroup: 1024,
            max_workgroups_per_dimension: 65_535,
            supports_subgroups: true,
            // Tensor/RT-core presence depends on the specific NVIDIA part; leave
            // false until a real compute-capability probe is wired.
            supports_coopmat: false,
            supports_rt_cores: false,
            warp_size: 32, // NVIDIA
        };

        let topology = MemoryTopology::Discrete { staging_required: true };
        let allocator = QualiaSlabAllocator::new(topology, capacity_bytes);

        // Pre-allocate the persistent device slab. All transient buffers are
        // sub-ranges of this allocation, addressed by offset.
        let slab = stream
            .alloc_zeros::<u8>(capacity_bytes)
            .map_err(|e| ForgeError::GpuUnavailable(format!("Failed to allocate CUDA slab: {:?}", e)))?;

        Ok(Self {
            ctx,
            stream,
            adapter,
            constraints,
            allocator,
            slab,
        })
    }

    pub fn allocate_and_write(
        &mut self,
        data: &[u8],
        binding: u32,
        group: u32,
    ) -> Result<BufferView, ForgeError> {
        // CUDA addresses everything through one slab via raw pointers, so the
        // wgpu usage class is not load-bearing here.
        let view = self.allocator.allocate_transient(data.len(), binding, group, BindingUsage::StorageReadWrite)?;
        if !data.is_empty() {
            let mut dst = self.slab.slice_mut(view.offset..view.offset + view.length_bytes);
            self.stream
                .memcpy_htod(data, &mut dst)
                .map_err(|e| ForgeError::GpuValidation(format!("H2D transfer failed: {:?}", e)))?;
        }
        Ok(view)
    }

    pub fn allocate_transient(
        &mut self,
        size_bytes: usize,
        binding: u32,
        group: u32,
    ) -> Result<BufferView, ForgeError> {
        self.allocator.allocate_transient(size_bytes, binding, group, BindingUsage::StorageReadWrite)
    }

    pub fn advance_read_head(&mut self, offset: usize) {
        self.allocator.advance_read_head(offset);
    }

    pub fn clear_transient_allocations(&mut self) {
        self.allocator.clear();
    }

    pub fn read_buffer_f32(&self, view: &BufferView) -> Result<Vec<f32>, ForgeError> {
        let src = self.slab.slice(view.offset..view.offset + view.length_bytes);
        let bytes = self
            .stream
            .clone_dtoh(&src)
            .map_err(|e| ForgeError::GpuValidation(format!("D2H transfer failed: {:?}", e)))?;

        let elements = view.length_bytes / std::mem::size_of::<f32>();
        let output = bytemuck::cast_slice::<u8, f32>(&bytes)[..elements].to_vec();
        Ok(output)
    }
}

#[cfg(feature = "cuda")]
pub struct CudaPipeline<'a> {
    context: &'a CudaComputeContext,
    func: CudaFunction,
    // Kept alive so the loaded function's backing module is not unloaded.
    _module: Arc<CudaModule>,
}

#[cfg(feature = "cuda")]
impl<'a> CudaPipeline<'a> {
    pub fn compile(
        context: &'a CudaComputeContext,
        source: &str,
        entry_point: &str,
    ) -> Result<Self, ForgeError> {
        let ptx = Ptx::from_src(source);
        let module = context
            .ctx
            .load_module(ptx)
            .map_err(|e| ForgeError::GpuValidation(format!("Failed to load PTX: {:?}", e)))?;
        let func = module
            .load_function(entry_point)
            .map_err(|e| ForgeError::GpuValidation(format!("Entry point not found in PTX module: {:?}", e)))?;

        Ok(Self {
            context,
            func,
            _module: module,
        })
    }
}

#[cfg(feature = "cuda")]
impl<'a> QualiaCompute for CudaPipeline<'a> {
    fn dispatch(
        &self,
        buffers: &[BufferView],
        schedule: &Schedule,
        element_count: usize,
    ) -> Result<u64, ForgeError> {
        let dispatch_x = schedule.dispatch_workgroups(element_count);

        let cfg = LaunchConfig {
            grid_dim: (dispatch_x, 1, 1),
            block_dim: (schedule.workgroup_size, 1, 1),
            shared_mem_bytes: 0,
        };

        if buffers.len() != 3 {
            return Err(ForgeError::GpuValidation(
                "CUDA affine-f32 expects exactly 3 buffers".to_string(),
            ));
        }

        // Resolve views by binding, not position, to match the emitter's layout
        // (input@0, output@1, params@2) regardless of the order they were passed.
        let view_for = |binding: u32| -> Result<&BufferView, ForgeError> {
            buffers.iter().find(|b| b.binding == binding).ok_or_else(|| {
                ForgeError::GpuValidation(format!("CUDA affine-f32 missing binding {binding}"))
            })
        };
        let input_view = view_for(0)?;
        let output_view = view_for(1)?;
        let params_view = view_for(2)?;

        // The uniform block lives in the slab; the PTX takes it by value, so we
        // copy the 16 bytes back to the host and forward them as a kernel arg.
        let params_host = {
            let pslice = self.context.slab.slice(params_view.offset..params_view.offset + 16);
            self.context
                .stream
                .clone_dtoh(&pslice)
                .map_err(|e| ForgeError::GpuValidation(format!("Failed to read params: {:?}", e)))?
        };
        let mut params = AffineParamsRaw { bytes: [0u8; 16] };
        params.bytes.copy_from_slice(&params_host[..16]);

        // Input/output are passed as raw device pointers (base + offset). The
        // SyncOnDrop guard keeps the slab pointer valid across the launch.
        let (base, _guard) = self.context.slab.device_ptr(&self.context.stream);
        let base = base as u64;
        let input_ptr = base + input_view.offset as u64;
        let output_ptr = base + output_view.offset as u64;

        let start = std::time::Instant::now();

        // Argument order MUST match the PTX `.entry` parameter declaration order,
        // which the emitter writes in buffer order: (input_ptr, output_ptr, params).
        // CUDA binds launch args to kernel params positionally, so the uniform
        // block is passed last — not first.
        let mut builder = self.context.stream.launch_builder(&self.func);
        builder.arg(&input_ptr);
        builder.arg(&output_ptr);
        builder.arg(&params);
        // Safety: argument count/types match the affine-f32 PTX signature
        // (input ptr, output ptr, params[16] by value); buffers are bounds-checked above.
        unsafe {
            builder
                .launch(cfg)
                .map_err(|e| ForgeError::GpuValidation(format!("CUDA launch failed: {:?}", e)))?;
        }

        self.context
            .stream
            .synchronize()
            .map_err(|e| ForgeError::GpuValidation(format!("CUDA sync failed: {:?}", e)))?;

        Ok(start.elapsed().as_nanos().min(u64::MAX as u128) as u64)
    }
}
