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
use super::oracle_ctx::OracleContext;
#[cfg(feature = "cuda")]
use crate::wgsl_forge::{
    emit_shader, AdapterConstraints, AdapterIdentity, BufferAccess, BufferElement, BufferSpec,
    ForgeError, KernelSpec, ScalarType, Schedule, TargetBackend,
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
        Self::from_source(context, &generated.source, &kernel.entry_point, kernel.clone())
    }

    /// Compile a *raw* CUDA-C source string (entry point + storage-buffer bindings
    /// supplied directly) to PTX via NVRTC and load it. This is for kernels that
    /// have no portable-IR analogue — notably the `nvcuda::wmma` tensor-core GEMM,
    /// whose f16/f32 fragment API and fixed 16x16x16 shape cannot be expressed in
    /// WGSL/IR. `storage_buffer_bindings` lists the kernel's pointer parameters in
    /// binding order (all treated as storage pointers; no by-value uniform).
    pub fn compile_cuda_c_source(
        context: &'a CudaComputeContext,
        source: &str,
        entry_point: &str,
        storage_buffer_bindings: &[u32],
    ) -> Result<Self, ForgeError> {
        let buffers: Vec<BufferSpec> = storage_buffer_bindings
            .iter()
            .map(|&binding| BufferSpec {
                group: 0,
                binding,
                name: format!("buf{binding}"),
                element: BufferElement::Scalar(ScalarType::F32),
                access: BufferAccess::StorageReadWrite,
            })
            .collect();
        let spec = KernelSpec {
            id: entry_point.to_string(),
            semantic_version: 1,
            entry_point: entry_point.to_string(),
            description: "raw CUDA-C kernel".to_string(),
            buffers,
            ops: Vec::new(),
            shared_memory: Vec::new(),
        };
        Self::from_source(context, source, entry_point, spec)
    }

    fn from_source(
        context: &'a CudaComputeContext,
        source: &str,
        entry_point: &str,
        spec: KernelSpec,
    ) -> Result<Self, ForgeError> {
        let ptx = nvrtc_compile_to_ptx(context, source)?;
        let module = context
            .ctx
            .load_module(ptx)
            .map_err(|e| ForgeError::GpuValidation(format!("Failed to load module: {:?}", e)))?;
        let func = module
            .load_function(entry_point)
            .map_err(|e| ForgeError::GpuValidation(format!("Entry point not found: {:?}", e)))?;

        Ok(Self {
            context,
            func,
            spec,
            _module: module,
        })
    }
}

/// Compiles a CUDA-C source string to a driver-loadable PTX module via NVRTC,
/// targeting the device's *actual* compute capability and making the CUDA toolkit
/// headers resolvable. NVRTC's default `--include-path` search list is empty, so
/// tensor-core kernels (`#include <mma.h>`) need the toolkit include dir passed
/// explicitly — without it NVRTC fails with "could not open source file mma.h".
#[cfg(feature = "cuda")]
fn nvrtc_compile_to_ptx(
    context: &CudaComputeContext,
    source: &str,
) -> Result<cudarc::nvrtc::Ptx, ForgeError> {
    use cudarc::nvrtc::{compile_ptx_with_opts, CompileOptions};
    let (major, minor) = context.ctx.compute_capability().map_err(|e| {
        ForgeError::GpuUnavailable(format!("compute-capability query failed: {:?}", e))
    })?;
    let mut include_paths = Vec::new();
    if let Ok(cuda_path) = std::env::var("CUDA_PATH") {
        include_paths.push(format!("{cuda_path}/include"));
    }
    let opts = CompileOptions {
        arch: Some(arch_for_capability(major, minor)),
        include_paths,
        ..Default::default()
    };
    let compiled = compile_ptx_with_opts(source, opts)
        .map_err(|e| ForgeError::GpuValidation(format!("NVRTC compile failed: {:?}", e)))?;
    // The installed nvrtc can be newer than the driver, which then rejects the PTX
    // ISA version. Our kernels use only long-stable instructions (incl. the stable
    // WMMA `mma.sync`), so rewrite `.version` down to one the driver supports.
    Ok(cudarc::nvrtc::Ptx::from_src(downgrade_ptx_isa(&compiled.to_src())))
}

/// Maps a CUDA compute capability to the `--gpu-architecture=compute_XX` virtual
/// arch NVRTC should target. Floors unknown/older parts to `compute_70` — the
/// minimum for WMMA tensor-core ops; the driver JIT-upgrades the emitted PTX to the
/// real arch, so this stays correct (if not arch-optimal) on newer cards.
#[cfg(feature = "cuda")]
fn arch_for_capability(major: i32, minor: i32) -> &'static str {
    match (major, minor) {
        (9, 0) => "compute_90",
        (8, 9) => "compute_89",
        (8, 7) => "compute_87",
        (8, 6) => "compute_86",
        (8, 0) => "compute_80",
        (7, 5) => "compute_75",
        (7, 2) => "compute_72",
        (7, 0) => "compute_70",
        _ => "compute_70",
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

        // A post-launch synchronize failure is a device-level fault; surface it as
        // the unified DeviceLost rather than a generic validation error (plan §7).
        self.context
            .stream
            .synchronize()
            .map_err(|e| ForgeError::DeviceLost(format!("CUDA sync failed: {:?}", e)))?;

        Ok(start.elapsed().as_nanos().min(u64::MAX as u128) as u64)
    }
}

#[cfg(feature = "cuda")]
impl OracleContext for CudaComputeContext {
    fn allocate_and_write(
        &mut self,
        data: &[u8],
        binding: u32,
        group: u32,
        _usage: BindingUsage,
    ) -> Result<BufferView, ForgeError> {
        // CUDA addresses one slab through raw pointers, so the binding usage is not
        // load-bearing; defer to the existing 3-arg inherent method verbatim.
        CudaComputeContext::allocate_and_write(self, data, binding, group)
    }

    fn allocate_transient(
        &mut self,
        size_bytes: usize,
        binding: u32,
        group: u32,
        _usage: BindingUsage,
    ) -> Result<BufferView, ForgeError> {
        CudaComputeContext::allocate_transient(self, size_bytes, binding, group)
    }

    fn read_buffer_f32(&self, view: &BufferView) -> Result<Vec<f32>, ForgeError> {
        CudaComputeContext::read_buffer_f32(self, view)
    }

    fn clear_transient_allocations(&mut self) {
        CudaComputeContext::clear_transient_allocations(self);
    }

    fn adapter(&self) -> &AdapterIdentity {
        &self.adapter
    }

    fn constraints(&self) -> &AdapterConstraints {
        &self.constraints
    }

    fn timestamp_supported(&self) -> bool {
        // The CUDA backend times on the host wall clock (see [`CudaPipeline::dispatch`]);
        // there is no GPU-timestamp query path here.
        false
    }

    /// Compile the kernel's CUDA-C (NVRTC → PTX, emitted internally by
    /// [`CudaPipeline::compile_cuda_c`]) and run the warmup + timed-sample dispatch
    /// loop. Mirrors the wgpu loop shape so the generic oracle is backend-agnostic;
    /// the cross-backend CUDA oracle uses `warmups = 0, samples = 1`, reproducing the
    /// single dispatch the previous `evaluate_*_cuda` functions performed.
    fn run_kernel(
        &mut self,
        kernel: &KernelSpec,
        schedule: &Schedule,
        buffers: &[BufferView],
        element_count: usize,
        warmups: usize,
        samples: usize,
    ) -> Result<Vec<u64>, ForgeError> {
        let pipeline = CudaPipeline::compile_cuda_c(self, kernel, *schedule)?;

        for _ in 0..warmups {
            pipeline.dispatch(buffers, schedule, element_count)?;
        }
        let mut timing_samples = Vec::with_capacity(samples);
        for _ in 0..samples {
            timing_samples.push(pipeline.dispatch(buffers, schedule, element_count)?);
        }
        Ok(timing_samples)
    }
}
