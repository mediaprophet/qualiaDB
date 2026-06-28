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
use super::compute::QualiaCompute;
#[cfg(feature = "cuda")]
use super::memory::{BindingUsage, BufferView, MemoryTopology, QualiaSlabAllocator};
#[cfg(feature = "cuda")]
use crate::wgsl_forge::{
    emit_shader, AdapterConstraints, AdapterIdentity, BufferAccess, ForgeError, KernelSpec,
    Schedule, TargetBackend,
};

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
    spec: KernelSpec,
    // Kept alive so the loaded function's backing module is not unloaded.
    _module: Arc<CudaModule>,
}

#[cfg(feature = "cuda")]
impl<'a> CudaPipeline<'a> {
    /// Emit CUDA-C for the kernel and compile it to PTX via NVRTC (mirrors the
    /// HLSL -> DXC path), then load the resulting module.
    pub fn compile_cuda_c(
        context: &'a CudaComputeContext,
        kernel: &KernelSpec,
        schedule: Schedule,
    ) -> Result<Self, ForgeError> {
        let generated = emit_shader(kernel, schedule, TargetBackend::CudaC)?;
        let compiled = cudarc::nvrtc::compile_ptx(&generated.source)
            .map_err(|e| ForgeError::GpuValidation(format!("NVRTC compile failed: {:?}", e)))?;
        // The installed toolkit (nvrtc) can be newer than the driver, in which case
        // the driver rejects the PTX ISA version. Our kernels only use long-stable
        // instructions, so we rewrite the `.version` directive down to one the
        // driver supports.
        let ptx = cudarc::nvrtc::Ptx::from_src(downgrade_ptx_isa(&compiled.to_src()));
        let module = context
            .ctx
            .load_module(ptx)
            .map_err(|e| ForgeError::GpuValidation(format!("Failed to load module: {:?}", e)))?;
        let func = module
            .load_function(&kernel.entry_point)
            .map_err(|e| ForgeError::GpuValidation(format!("Entry point not found: {:?}", e)))?;

        Ok(Self {
            context,
            func,
            spec: kernel.clone(),
            _module: module,
        })
    }
}

/// Rewrites the PTX `.version M.m` directive to a broadly-supported ISA so an
/// older driver can JIT NVRTC output from a newer toolkit. Our emitted kernels
/// use only long-stable instructions (fma, tanh.approx, shared memory, bar.sync),
/// which are valid at ISA 8.0.
#[cfg(feature = "cuda")]
fn downgrade_ptx_isa(ptx: &str) -> String {
    const TARGET_VERSION: &str = ".version 8.0";
    let mut out = String::with_capacity(ptx.len());
    let mut replaced = false;
    for line in ptx.lines() {
        if !replaced && line.trim_start().starts_with(".version") {
            out.push_str(TARGET_VERSION);
            replaced = true;
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
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

        // Build launch args from the kernel spec, in binding order: each storage
        // buffer becomes a device pointer, and the single uniform block is passed
        // by value last — matching the CUDA-C signature emitted by emit_cuda_c.
        let (base, _guard) = self.context.slab.device_ptr(&self.context.stream);
        let base = base as u64;

        let mut sorted = self.spec.buffers.clone();
        sorted.sort_by_key(|b| b.binding);

        let mut ptr_args: Vec<u64> = Vec::with_capacity(sorted.len());
        let mut params: Option<AffineParamsRaw> = None;
        for bspec in &sorted {
            let view = buffers
                .iter()
                .find(|b| b.binding == bspec.binding)
                .ok_or_else(|| {
                    ForgeError::GpuValidation(format!("CUDA dispatch missing binding {}", bspec.binding))
                })?;
            if bspec.access == BufferAccess::Uniform {
                let host = self
                    .context
                    .stream
                    .clone_dtoh(&self.context.slab.slice(view.offset..view.offset + 16))
                    .map_err(|e| ForgeError::GpuValidation(format!("Failed to read params: {:?}", e)))?;
                let mut blob = AffineParamsRaw { bytes: [0u8; 16] };
                blob.bytes.copy_from_slice(&host[..16]);
                params = Some(blob);
            } else {
                ptr_args.push(base + view.offset as u64);
            }
        }

        let start = std::time::Instant::now();
        let mut builder = self.context.stream.launch_builder(&self.func);
        for ptr in &ptr_args {
            builder.arg(ptr);
        }
        if let Some(params) = &params {
            builder.arg(params);
        }
        // Safety: argument count/types match the CUDA-C signature emitted for
        // this kernel (storage pointers in binding order, then the uniform block).
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
