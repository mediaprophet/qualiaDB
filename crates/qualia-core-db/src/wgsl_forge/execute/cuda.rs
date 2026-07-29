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
use std::sync::{Arc, Mutex, OnceLock};

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
#[cfg(feature = "cuda")]
use cudarc::driver::{
    CudaContext, CudaFunction, CudaGraph, CudaModule, CudaSlice, CudaStream, DevicePtr, DeviceRepr,
    LaunchConfig, PushKernelArg,
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
    /// Secondary stream for overlapping H2D parameter writes with compute.
    /// Lazily created on first `write_view_prefetch` call to avoid overhead
    /// when double-buffering is not used.
    pub prefetch_stream: Option<Arc<CudaStream>>,
    /// Cache of loaded CUDA functions keyed by (source_hash, entry_point).
    /// Avoids redundant `load_module` JIT on every `compile_pipe!` call —
    /// the PTX text is already cached in `NVRTC_PTX_CACHE`, but the driver
    /// module load is a separate JIT step that was repeated per token.
    pub module_cache:
        Mutex<std::collections::HashMap<u64, (CudaFunction, Arc<CudaModule>, Arc<KernelSpec>)>>,
    pub adapter: AdapterIdentity,
    pub constraints: AdapterConstraints,
    pub allocator: QualiaSlabAllocator,
    pub slab: CudaSlice<u8>,
}

/// Instantiated CUDA graph owned by a prepared runtime.
///
/// CUDA graph objects require external serialization. Qualia stores this wrapper only inside
/// the `MultiWeightDevice` mutex, so moving it with that device is safe while concurrent access
/// remains impossible.
#[cfg(feature = "cuda")]
pub struct CapturedCudaGraph {
    graph: CudaGraph,
}

#[cfg(feature = "cuda")]
impl CapturedCudaGraph {
    /// Exact number of nodes retained by the captured CUDA graph.
    pub fn node_count(&self) -> Result<usize, ForgeError> {
        let mut count = 0usize;
        let status = unsafe {
            cudarc::driver::sys::cuGraphGetNodes(
                self.graph.cu_graph(),
                core::ptr::null_mut(),
                &mut count,
            )
        };
        if status == cudarc::driver::sys::CUresult::CUDA_SUCCESS {
            Ok(count)
        } else {
            Err(ForgeError::GpuValidation(format!(
                "CUDA graph node query failed: {status:?}"
            )))
        }
    }
}

// SAFETY: all access is serialized by `multi_weight_device()`'s mutex; cudarc's graph retains
// the stream and context that own the captured operations.
#[cfg(feature = "cuda")]
unsafe impl Send for CapturedCudaGraph {}

#[cfg(feature = "cuda")]
impl CudaComputeContext {
    pub fn new(capacity_bytes: usize) -> Result<Self, ForgeError> {
        let ctx = CudaContext::new(0)
            .map_err(|e| ForgeError::GpuUnavailable(format!("CUDA init failed: {:?}", e)))?;
        // Forge owns stream ordering explicitly (`join_prefetch` before every dependent launch).
        // cudarc's cross-stream event injection is therefore redundant and cannot be introduced
        // while a CUDA stream is being captured.
        //
        // SAFETY: the slab outlives both streams; every prefetch write is joined before compute;
        // final readback synchronizes the compute stream before the context can be dropped.
        unsafe {
            ctx.disable_event_tracking();
        }
        // CUDA graph capture is unsupported on the legacy default stream. A dedicated
        // non-blocking stream also makes ordering ownership explicit for prepared inference.
        let stream = ctx
            .new_stream()
            .map_err(|e| ForgeError::GpuUnavailable(format!("CUDA stream init failed: {e:?}")))?;

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

        let topology = MemoryTopology::Discrete {
            staging_required: true,
        };
        let allocator = QualiaSlabAllocator::new(topology, capacity_bytes);

        // Pre-allocate the persistent device slab. All transient buffers are
        // sub-ranges of this allocation, addressed by offset.
        let slab = stream.alloc_zeros::<u8>(capacity_bytes).map_err(|e| {
            ForgeError::GpuUnavailable(format!("Failed to allocate CUDA slab: {:?}", e))
        })?;

        Ok(Self {
            ctx,
            stream,
            prefetch_stream: None,
            module_cache: Mutex::new(std::collections::HashMap::new()),
            adapter,
            constraints,
            allocator,
            slab,
        })
    }

    /// Begin thread-local capture on the prepared compute stream.
    pub fn begin_graph_capture(&self) -> Result<(), ForgeError> {
        // Capture may not inherit event-tracked dependencies from setup work. Drain all cold
        // uploads/module preparation before establishing the graph boundary.
        self.stream.synchronize().map_err(|e| {
            ForgeError::GpuValidation(format!("CUDA graph pre-capture sync: {e:?}"))
        })?;
        self.stream
            .begin_capture(
                cudarc::driver::sys::CUstreamCaptureMode::CU_STREAM_CAPTURE_MODE_THREAD_LOCAL,
            )
            .map_err(|e| ForgeError::GpuValidation(format!("CUDA graph begin capture: {e:?}")))
    }

    /// Finish, instantiate and upload the current compute-stream capture.
    pub fn end_graph_capture(&self) -> Result<CapturedCudaGraph, ForgeError> {
        let graph = self
            .stream
            .end_capture(
                cudarc::driver::sys::CUgraphInstantiate_flags::CUDA_GRAPH_INSTANTIATE_FLAG_USE_NODE_PRIORITY,
            )
            .map_err(|e| ForgeError::GpuValidation(format!("CUDA graph end capture: {e:?}")))?
            .ok_or_else(|| ForgeError::GpuValidation("CUDA capture produced no graph".into()))?;
        Ok(CapturedCudaGraph { graph })
    }

    /// Enqueue one replay on the graph's retained compute stream.
    pub fn launch_graph(&self, graph: &CapturedCudaGraph) -> Result<(), ForgeError> {
        graph
            .graph
            .launch()
            .map_err(|e| ForgeError::GpuValidation(format!("CUDA graph launch: {e:?}")))
    }

    pub fn allocate_and_write(
        &mut self,
        data: &[u8],
        binding: u32,
        group: u32,
    ) -> Result<BufferView, ForgeError> {
        // CUDA addresses everything through one slab via raw pointers, so the
        // wgpu usage class is not load-bearing here.
        let view = self.allocator.allocate_transient(
            data.len(),
            binding,
            group,
            BindingUsage::StorageReadWrite,
        )?;
        if !data.is_empty() {
            let mut dst = self
                .slab
                .slice_mut(view.offset..view.offset + view.length_bytes);
            self.stream
                .memcpy_htod(data, &mut dst)
                .map_err(|e| ForgeError::GpuValidation(format!("H2D transfer failed: {:?}", e)))?;
        }
        Ok(view)
    }

    /// Lazily create the prefetch stream if it doesn't exist yet.
    fn ensure_prefetch_stream(&mut self) -> Result<&Arc<CudaStream>, ForgeError> {
        if self.prefetch_stream.is_none() {
            let s = self
                .ctx
                .new_stream()
                .map_err(|e| ForgeError::GpuUnavailable(format!("prefetch stream: {:?}", e)))?;
            self.prefetch_stream = Some(s);
        }
        Ok(self.prefetch_stream.as_ref().unwrap())
    }

    /// Overwrite a device view with host bytes on the **prefetch stream**,
    /// overlapping with compute on the primary stream. Caller must invoke
    /// [`join_prefetch`] before launching a kernel that reads this data.
    pub fn write_view_prefetch(
        &mut self,
        view: &BufferView,
        data: &[u8],
    ) -> Result<(), ForgeError> {
        if data.len() > view.length_bytes {
            return Err(ForgeError::GpuValidation(format!(
                "write_view_prefetch overflow: {} > {}",
                data.len(),
                view.length_bytes
            )));
        }
        if data.is_empty() {
            return Ok(());
        }
        let pf_stream = self.ensure_prefetch_stream()?.clone();
        let mut dst = self.slab.slice_mut(view.offset..view.offset + data.len());
        pf_stream
            .memcpy_htod(data, &mut dst)
            .map_err(|e| ForgeError::GpuValidation(format!("prefetch H2D: {:?}", e)))?;
        Ok(())
    }

    /// Make the compute stream wait for all outstanding prefetch-stream work.
    /// Call this before launching a kernel that depends on prefetched data.
    pub fn join_prefetch(&self) -> Result<(), ForgeError> {
        if let Some(ref pf) = self.prefetch_stream {
            self.stream
                .join(pf)
                .map_err(|e| ForgeError::GpuValidation(format!("join_prefetch: {:?}", e)))?;
        }
        Ok(())
    }

    /// Overwrite an existing device view with host bytes (no new allocation).
    /// `data.len()` must be ≤ `view.length_bytes`.
    pub fn write_view(&mut self, view: &BufferView, data: &[u8]) -> Result<(), ForgeError> {
        if data.len() > view.length_bytes {
            return Err(ForgeError::GpuValidation(format!(
                "write_view overflow: {} > {}",
                data.len(),
                view.length_bytes
            )));
        }
        if data.is_empty() {
            return Ok(());
        }
        let mut dst = self.slab.slice_mut(view.offset..view.offset + data.len());
        self.stream
            .memcpy_htod(data, &mut dst)
            .map_err(|e| ForgeError::GpuValidation(format!("H2D write_view failed: {:?}", e)))?;
        Ok(())
    }

    pub fn allocate_transient(
        &mut self,
        size_bytes: usize,
        binding: u32,
        group: u32,
    ) -> Result<BufferView, ForgeError> {
        self.allocator.allocate_transient(
            size_bytes,
            binding,
            group,
            BindingUsage::StorageReadWrite,
        )
    }

    pub fn advance_read_head(&mut self, offset: usize) {
        self.allocator.advance_read_head(offset);
    }

    pub fn clear_transient_allocations(&mut self) {
        self.allocator.clear();
    }

    /// See [`QualiaSlabAllocator::write_checkpoint`].
    pub fn write_checkpoint(&self) -> u64 {
        self.allocator.write_checkpoint()
    }

    /// See [`QualiaSlabAllocator::restore_checkpoint`].
    pub fn restore_checkpoint(&mut self, write_count: u64) {
        self.allocator.restore_checkpoint(write_count);
    }

    pub fn read_buffer_f32(&self, view: &BufferView) -> Result<Vec<f32>, ForgeError> {
        let src = self
            .slab
            .slice(view.offset..view.offset + view.length_bytes);
        let bytes = self
            .stream
            .clone_dtoh(&src)
            .map_err(|e| ForgeError::GpuValidation(format!("D2H transfer failed: {:?}", e)))?;

        let elements = view.length_bytes / std::mem::size_of::<f32>();
        let output = bytemuck::cast_slice::<u8, f32>(&bytes)[..elements].to_vec();
        Ok(output)
    }

    /// Copy a device view into a caller-owned `u32` slice.
    ///
    /// Unlike [`Self::read_buffer_f32`], this performs no host allocation. It is the decode
    /// token-readback boundary: the four-byte copy also synchronizes all preceding stream work.
    pub fn read_buffer_u32_into(
        &self,
        view: &BufferView,
        output: &mut [u32],
    ) -> Result<(), ForgeError> {
        let bytes = bytemuck::cast_slice_mut(output);
        if bytes.len() > view.length_bytes {
            return Err(ForgeError::GpuValidation(format!(
                "u32 readback overflow: {} > {}",
                bytes.len(),
                view.length_bytes
            )));
        }
        let src = self.slab.slice(view.offset..view.offset + bytes.len());
        self.stream
            .memcpy_dtoh(&src, bytes)
            .map_err(|e| ForgeError::GpuValidation(format!("D2H transfer failed: {e:?}")))
    }

    /// Double-precision readback, the `f64` mirror of [`Self::read_buffer_f32`]
    /// (8 bytes/elem). Used by the native CUDA-f64 GEMM path — WGSL has no `f64`,
    /// so this is CUDA-only by construction.
    pub fn read_buffer_f64(&self, view: &BufferView) -> Result<Vec<f64>, ForgeError> {
        let src = self
            .slab
            .slice(view.offset..view.offset + view.length_bytes);
        let bytes = self
            .stream
            .clone_dtoh(&src)
            .map_err(|e| ForgeError::GpuValidation(format!("D2H transfer failed: {:?}", e)))?;

        let elements = view.length_bytes / std::mem::size_of::<f64>();
        let output = bytemuck::cast_slice::<u8, f64>(&bytes)[..elements].to_vec();
        Ok(output)
    }
}

#[cfg(feature = "cuda")]
pub struct CudaPipeline<'a> {
    context: &'a CudaComputeContext,
    func: CudaFunction,
    spec: Arc<KernelSpec>,
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
        Self::from_source(
            context,
            &generated.source,
            &kernel.entry_point,
            kernel.clone(),
        )
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
        let ptx = nvrtc_compile_to_ptx_cached(context, source)?;
        Self::from_ptx(context, &ptx, entry_point, spec)
    }

    /// Load a pipeline from already-compiled PTX (no NVRTC). Used by the process-wide
    /// WMMA cache path so hot GEMM calls only pay `load_module`.
    pub fn from_ptx(
        context: &'a CudaComputeContext,
        ptx: &cudarc::nvrtc::Ptx,
        entry_point: &str,
        spec: KernelSpec,
    ) -> Result<Self, ForgeError> {
        let module = context
            .ctx
            .load_module(ptx.clone())
            .map_err(|e| ForgeError::GpuValidation(format!("Failed to load module: {:?}", e)))?;
        let func = module
            .load_function(entry_point)
            .map_err(|e| ForgeError::GpuValidation(format!("Entry point not found: {:?}", e)))?;

        Ok(Self {
            context,
            func,
            spec: Arc::new(spec),
            _module: module,
        })
    }

    /// Compile (or reuse cached PTX for) raw CUDA-C and load — same as
    /// [`compile_cuda_c_source`] but shares the process NVRTC cache when `source`
    /// matches a previously compiled kernel body.
    pub fn compile_cuda_c_source_cached(
        context: &'a CudaComputeContext,
        source: &str,
        entry_point: &str,
        storage_buffer_bindings: &[u32],
    ) -> Result<Self, ForgeError> {
        let src_hash = fnv1a64_bytes(source.as_bytes());
        let cache_key = src_hash ^ fnv1a64_bytes(entry_point.as_bytes()).rotate_left(1);

        // Fast path: function already loaded — skip NVRTC + load_module entirely.
        // Zero allocations: just Arc clones (atomic increments).
        if let Ok(guard) = context.module_cache.lock() {
            if let Some((func, module, spec)) = guard.get(&cache_key) {
                return Ok(Self {
                    context,
                    func: func.clone(),
                    spec: spec.clone(),
                    _module: module.clone(),
                });
            }
        }

        // Slow path: NVRTC compile + load_module — construct full KernelSpec.
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
            description: "raw CUDA-C kernel (cached PTX)".to_string(),
            buffers,
            ops: Vec::new(),
            shared_memory: Vec::new(),
        };
        let ptx = nvrtc_compile_to_ptx_cached(context, source)?;
        let pipe = Self::from_ptx(context, &ptx, entry_point, spec)?;

        // Store the loaded function + spec so subsequent calls skip load_module.
        if let Ok(mut guard) = context.module_cache.lock() {
            guard.insert(
                cache_key,
                (pipe.func.clone(), pipe._module.clone(), pipe.spec.clone()),
            );
        }
        Ok(pipe)
    }

    /// Load a hand-emitted PTX module (from `emit/ptx.rs`) directly into the CUDA
    /// driver — no NVRTC compilation step. This is the PTX execution bridge:
    /// the emitter produces complete PTX text with `.version`, `.target`,
    /// `.address_size`, entry point, and full kernel body; the driver JITs it
    /// to the actual GPU ISA.
    ///
    /// Shared-memory size is passed via `LaunchConfig.shared_mem_bytes` at
    /// dispatch time, not at compile time.
    pub fn compile_ptx(
        context: &'a CudaComputeContext,
        ptx_source: &str,
        entry_point: &str,
        storage_buffer_bindings: &[u32],
    ) -> Result<Self, ForgeError> {
        let src_hash = fnv1a64_bytes(ptx_source.as_bytes());
        let cache_key = src_hash ^ fnv1a64_bytes(entry_point.as_bytes()).rotate_left(1);

        // Fast path: function already loaded — skip load_module entirely.
        if let Ok(guard) = context.module_cache.lock() {
            if let Some((func, module, spec)) = guard.get(&cache_key) {
                return Ok(Self {
                    context,
                    func: func.clone(),
                    spec: spec.clone(),
                    _module: module.clone(),
                });
            }
        }

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
            description: "hand-emitted PTX kernel".to_string(),
            buffers,
            ops: Vec::new(),
            shared_memory: Vec::new(),
        };
        let ptx = cudarc::nvrtc::Ptx::from_src(ptx_source.to_string());
        let pipe = Self::from_ptx(context, &ptx, entry_point, spec)?;

        if let Ok(mut guard) = context.module_cache.lock() {
            guard.insert(
                cache_key,
                (pipe.func.clone(), pipe._module.clone(), pipe.spec.clone()),
            );
        }
        Ok(pipe)
    }
}

/// Compiles a CUDA-C source string to a driver-loadable PTX module via NVRTC,
/// targeting the device's *actual* compute capability and making the CUDA toolkit
/// headers resolvable. NVRTC's default `--include-path` search list is empty, so
/// tensor-core kernels (`#include <mma.h>`) need the toolkit include dir passed
/// explicitly — without it NVRTC fails with "could not open source file mma.h".
/// Process-wide cache: (source_hash, arch, ptx_text) so NVRTC runs once per kernel/arch.
#[cfg(feature = "cuda")]
static NVRTC_PTX_CACHE: OnceLock<Mutex<std::collections::HashMap<(u64, String), String>>> =
    OnceLock::new();

#[cfg(feature = "cuda")]
fn nvrtc_ptx_cache() -> &'static Mutex<std::collections::HashMap<(u64, String), String>> {
    NVRTC_PTX_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

/// Hash of CUDA-C source body (FNV-1a 64) for cache keys — no heap string key.
#[cfg(feature = "cuda")]
fn fnv1a64_bytes(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Compile CUDA-C → PTX with process-wide cache (key = source FNV + compute arch).
#[cfg(feature = "cuda")]
pub(crate) fn nvrtc_compile_to_ptx_cached(
    context: &CudaComputeContext,
    source: &str,
) -> Result<cudarc::nvrtc::Ptx, ForgeError> {
    use cudarc::nvrtc::{compile_ptx_with_opts, CompileOptions};
    let (major, minor) = context.ctx.compute_capability().map_err(|e| {
        ForgeError::GpuUnavailable(format!("compute-capability query failed: {:?}", e))
    })?;
    let arch = arch_for_capability(major, minor).to_string();
    let key = (fnv1a64_bytes(source.as_bytes()), arch.clone());

    if let Ok(guard) = nvrtc_ptx_cache().lock() {
        if let Some(text) = guard.get(&key) {
            return Ok(cudarc::nvrtc::Ptx::from_src(text.clone()));
        }
    }

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
    let text = downgrade_ptx_isa(&compiled.to_src());
    if let Ok(mut guard) = nvrtc_ptx_cache().lock() {
        guard.insert(key, text.clone());
        log::info!(
            "cuda_nvrtc|cache_store|arch={arch}|src_hash={:#x}|ptx_bytes={}",
            fnv1a64_bytes(source.as_bytes()),
            text.len()
        );
    }
    Ok(cudarc::nvrtc::Ptx::from_src(text))
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
impl<'a> CudaPipeline<'a> {
    /// Launch without a host fence. Same-stream kernels stay ordered; the next
    /// `read_buffer_*` / `synchronize` is the completion barrier. Used by the
    /// P4 decode attention chain to avoid one PCIe-class fence per micro-kernel.
    pub fn dispatch_async(
        &self,
        buffers: &[BufferView],
        schedule: &Schedule,
        element_count: usize,
    ) -> Result<(), ForgeError> {
        self.launch_inner(buffers, schedule, element_count, false)
            .map(|_| ())
    }

    /// Launch a PTX kernel with shared-memory size and a 3D grid/block config.
    /// Used by hand-emitted PTX kernels (RMSNorm, Q4K GEMV, WMMA GEMV, SDPA)
    /// that need `shared_mem_bytes` and multi-dimensional dispatch.
    pub fn dispatch_ptx(
        &self,
        buffers: &[BufferView],
        grid: (u32, u32, u32),
        block: (u32, u32, u32),
        shared_mem_bytes: u32,
    ) -> Result<(), ForgeError> {
        let cfg = LaunchConfig {
            grid_dim: grid,
            block_dim: block,
            shared_mem_bytes,
        };

        let (base, _guard) = self.context.slab.device_ptr(&self.context.stream);
        let base = base as u64;

        let mut ptr_args: [u64; 16] = [0; 16];
        let n_bufs = buffers.len().min(16);
        for i in 0..n_bufs {
            ptr_args[i] = base + buffers[i].offset as u64;
        }

        let mut builder = self.context.stream.launch_builder(&self.func);
        for i in 0..n_bufs {
            builder.arg(&ptr_args[i]);
        }
        unsafe {
            builder
                .launch(cfg)
                .map_err(|e| ForgeError::GpuValidation(format!("PTX launch failed: {:?}", e)))?;
        }
        Ok(())
    }

    /// Fast-path async dispatch for pre-sorted buffer views.
    ///
    /// Assumes `buffers` are already in ascending binding order (as the mega-pass
    /// always provides). Skips `spec.buffers.clone()` + sort + linear search —
    /// eliminating 2 Vec allocations and O(n²) search per dispatch.
    pub fn dispatch_async_sorted(
        &self,
        buffers: &[BufferView],
        schedule: &Schedule,
        element_count: usize,
    ) -> Result<(), ForgeError> {
        let dispatch_x = schedule.dispatch_workgroups(element_count);
        let cfg = LaunchConfig {
            grid_dim: (dispatch_x, 1, 1),
            block_dim: (schedule.workgroup_size, 1, 1),
            shared_mem_bytes: 0,
        };

        let (base, _guard) = self.context.slab.device_ptr(&self.context.stream);
        let base = base as u64;

        // Build pointer args directly from pre-sorted buffer views — no clone,
        // no sort, no linear search. Stack array for typical binding counts.
        let mut ptr_args: [u64; 16] = [0; 16];
        let n_bufs = buffers.len().min(16);
        for i in 0..n_bufs {
            ptr_args[i] = base + buffers[i].offset as u64;
        }

        let mut builder = self.context.stream.launch_builder(&self.func);
        for i in 0..n_bufs {
            builder.arg(&ptr_args[i]);
        }
        unsafe {
            builder
                .launch(cfg)
                .map_err(|e| ForgeError::GpuValidation(format!("CUDA launch failed: {:?}", e)))?;
        }
        Ok(())
    }

    /// Measure one pre-sorted kernel launch with CUDA events.
    ///
    /// This is a lab/profiling operation, not a decode hot-path primitive: creating and
    /// synchronizing timing events intentionally fences the stream. It remains useful when
    /// hardware performance counters are unavailable because the elapsed value is device time
    /// rather than host submission/synchronization wall time.
    pub fn dispatch_gpu_timed_ms_sorted(
        &self,
        buffers: &[BufferView],
        schedule: &Schedule,
        element_count: usize,
    ) -> Result<f32, ForgeError> {
        let dispatch_x = schedule.dispatch_workgroups(element_count);
        let cfg = LaunchConfig {
            grid_dim: (dispatch_x, 1, 1),
            block_dim: (schedule.workgroup_size, 1, 1),
            shared_mem_bytes: 0,
        };

        let (base, _guard) = self.context.slab.device_ptr(&self.context.stream);
        let base = base as u64;
        let mut ptr_args: [u64; 16] = [0; 16];
        let n_bufs = buffers.len().min(16);
        for index in 0..n_bufs {
            ptr_args[index] = base + buffers[index].offset as u64;
        }

        let mut builder = self.context.stream.launch_builder(&self.func);
        for ptr in ptr_args.iter().take(n_bufs) {
            builder.arg(ptr);
        }
        builder.record_kernel_launch(cudarc::driver::sys::CUevent_flags::CU_EVENT_DEFAULT);
        let events = unsafe {
            builder.launch(cfg).map_err(|error| {
                ForgeError::GpuValidation(format!("CUDA timed launch failed: {error:?}"))
            })?
        }
        .ok_or_else(|| {
            ForgeError::GpuValidation("CUDA timed launch returned no events".to_string())
        })?;
        events.0.elapsed_ms(&events.1).map_err(|error| {
            ForgeError::GpuValidation(format!("CUDA event timing failed: {error:?}"))
        })
    }

    fn launch_inner(
        &self,
        buffers: &[BufferView],
        schedule: &Schedule,
        element_count: usize,
        sync: bool,
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
                    ForgeError::GpuValidation(format!(
                        "CUDA dispatch missing binding {}",
                        bspec.binding
                    ))
                })?;
            if bspec.access == BufferAccess::Uniform {
                let host = self
                    .context
                    .stream
                    .clone_dtoh(&self.context.slab.slice(view.offset..view.offset + 16))
                    .map_err(|e| {
                        ForgeError::GpuValidation(format!("Failed to read params: {:?}", e))
                    })?;
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

        if sync {
            // A post-launch synchronize failure is a device-level fault; surface it as
            // the unified DeviceLost rather than a generic validation error (plan §7).
            self.context
                .stream
                .synchronize()
                .map_err(|e| ForgeError::DeviceLost(format!("CUDA sync failed: {:?}", e)))?;
        }

        Ok(start.elapsed().as_nanos().min(u64::MAX as u128) as u64)
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
        self.launch_inner(buffers, schedule, element_count, true)
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

#[cfg(all(test, feature = "cuda"))]
mod graph_tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

    #[test]
    fn captured_kernel_replays_without_host_redispatch() {
        let Ok(mut context) = CudaComputeContext::new(16 * 1024 * 1024) else {
            eprintln!("CUDA graph test skipped: CUDA context unavailable");
            return;
        };
        let Ok(mut value) = context.allocate_and_write(bytemuck::cast_slice(&[0u32; 1]), 0, 0)
        else {
            return;
        };
        let source = r#"
extern "C" __global__ void increment(unsigned *value) {
    if (blockIdx.x == 0u && threadIdx.x == 0u) value[0] += 1u;
}
"#;
        let Ok(pipeline) =
            CudaPipeline::compile_cuda_c_source_cached(&context, source, "increment", &[0])
        else {
            eprintln!("CUDA graph test skipped: NVRTC unavailable");
            return;
        };
        value.binding = 0;
        let schedule = Schedule {
            workgroup_size: 32,
            ..Default::default()
        };
        context.begin_graph_capture().unwrap();
        pipeline
            .dispatch_async_sorted(&[value], &schedule, 32)
            .unwrap();
        let graph = context.end_graph_capture().unwrap();
        assert_eq!(graph.node_count().unwrap(), 1);
        context.launch_graph(&graph).unwrap();
        context.launch_graph(&graph).unwrap();
        let mut output = [0u32; 1];
        context.read_buffer_u32_into(&value, &mut output).unwrap();
        assert_eq!(output[0], 2);

        assert_zero_alloc("cuda_graph_dynamic_h2d", || {
            context
                .write_view(&value, bytemuck::cast_slice(&[0u32; 1]))
                .unwrap();
        });
        assert_zero_alloc("cuda_graph_launch", || {
            context.launch_graph(&graph).unwrap();
        });
        assert_zero_alloc("cuda_graph_token_d2h", || {
            context.read_buffer_u32_into(&value, &mut output).unwrap();
        });
    }
}
