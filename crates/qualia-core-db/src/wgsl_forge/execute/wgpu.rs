use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Instant;

use super::compute::QualiaCompute;
use super::memory::{
    BindingUsage, BufferView, MemoryTopology, QualiaSlabAllocator, DEFAULT_BINDING_ALIGNMENT,
};
use super::oracle_ctx::OracleContext;
#[cfg(not(target_arch = "wasm32"))]
use crate::gpu_context::GpuAdapterCaps;
use crate::wgsl_forge::{
    emit_shader, AdapterConstraints, AdapterIdentity, ForgeError, HardwareProfile, KernelSpec,
    Schedule, TargetBackend,
};

/// Create a wgpu instance respecting `QUALIA_WGPU_BACKEND`.
///
/// On Windows the default is DX12, but cooperative matrix (`VK_KHR_cooperative_matrix`)
/// is only exposed on the Vulkan backend for NVIDIA hardware. Setting
/// `QUALIA_WGPU_BACKEND=vulkan` routes the forge through Vulkan, un-gating coopmat.
#[cfg(not(target_arch = "wasm32"))]
fn create_instance() -> wgpu::Instance {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    if let Some(backends) = crate::gpu_context::qualia_backend_override() {
        desc.backends = backends;
    } else if cfg!(target_os = "windows") {
        desc.backends = wgpu::Backends::DX12;
    }
    wgpu::Instance::new(desc)
}

/// Like [`create_instance`] but forces Vulkan when the default backend lacks coopmat.
///
/// This is the un-gating path: on Windows/DX12 where `EXPERIMENTAL_COOPERATIVE_MATRIX`
/// is not advertised, we try Vulkan explicitly. NVIDIA Vulkan drivers expose
/// `VK_KHR_cooperative_matrix` which wgpu maps to `EXPERIMENTAL_COOPERATIVE_MATRIX`.
#[cfg(not(target_arch = "wasm32"))]
fn create_instance_for_coopmat() -> wgpu::Instance {
    // First try the user's explicit backend choice.
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    if let Some(backends) = crate::gpu_context::qualia_backend_override() {
        desc.backends = backends;
    } else {
        // No explicit override: try Vulkan first (coopmat is available there),
        // falling back to all backends if Vulkan isn't present.
        desc.backends = wgpu::Backends::VULKAN;
    }
    wgpu::Instance::new(desc)
}

/// Find the best adapter for cooperative matrix work.
///
/// Enumerates adapters on the given instance and returns the first that
/// advertises `EXPERIMENTAL_COOPERATIVE_MATRIX`, preferring discrete GPUs.
/// Returns `None` if no adapter has coopmat.
#[cfg(not(target_arch = "wasm32"))]
fn find_coopmat_adapter(instance: &wgpu::Instance) -> Option<wgpu::Adapter> {
    let backends = if let Some(b) = crate::gpu_context::qualia_backend_override() {
        b
    } else {
        wgpu::Backends::all()
    };
    let adapters = pollster::block_on(instance.enumerate_adapters(backends));
    // First pass: look for a discrete GPU with coopmat.
    for adapter in &adapters {
        let info = adapter.get_info();
        if info.device_type != wgpu::DeviceType::DiscreteGpu {
            continue;
        }
        let features = adapter.features();
        if features.contains(wgpu::Features::EXPERIMENTAL_COOPERATIVE_MATRIX) {
            return Some(adapter.clone());
        }
    }
    // Second pass: any adapter with coopmat.
    for adapter in &adapters {
        let features = adapter.features();
        if features.contains(wgpu::Features::EXPERIMENTAL_COOPERATIVE_MATRIX) {
            return Some(adapter.clone());
        }
    }
    None
}

pub struct WgpuComputeContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: AdapterIdentity,
    pub constraints: AdapterConstraints,
    /// Rich topology description for `profile-hardware` and cache keying.
    pub profile: HardwareProfile,
    pub allocator: QualiaSlabAllocator,
    /// Backs read-only storage and uniform views (both non-exclusive usages,
    /// so they may share one buffer).
    pub slab: wgpu::Buffer,
    /// Backs read-write storage outputs. wgpu treats read-write storage as an
    /// exclusive usage, so it cannot share a buffer with the read-only inputs in
    /// the same dispatch.
    pub out_slab: wgpu::Buffer,
    /// Backs the **persistent weight region** ([`BindingUsage::StorageReadResident`]): big,
    /// upload-once matrices (a decode layer's projection / FFN weights) that are referenced by
    /// offset across many `run`s instead of being re-uploaded each call. Separate buffer from the
    /// transient ring so [`Self::clear_transient_allocations`] never recycles it.
    pub weight_slab: wgpu::Buffer,
    /// Write-once bump cursor into `weight_slab` (bytes, kept 256-aligned). Weights are never
    /// freed individually; [`Self::clear_weights`] resets it to reuse the region for a new model.
    weight_cursor: usize,
    pub timestamp_supported: bool,
    pub timestamp_period_ns: f32,
    timestamp_resources: Option<TimestampResources>,
    /// Process-lifetime cache of compiled compute pipelines, keyed by `entry\0source`.
    /// Pipeline creation (shader compile + PSO build) is the dominant per-node cost once
    /// the device + slab are reused; a graph re-run over the same kernels (e.g. one decode
    /// block per generated token) then pays zero compile after the first. Survives
    /// [`clear_transient_allocations`](Self::clear_transient_allocations) — pipelines reference
    /// the shader + bind-group *layout*, never the slab buffers (bind groups are rebuilt per
    /// call). `RefCell` because [`compile_pipeline_cached`](Self::compile_pipeline_cached) takes
    /// `&self`; the context is used single-threaded per dispatch (shared dispatch is serialized
    /// behind a `Mutex` by the dispatcher).
    pipeline_cache: RefCell<HashMap<String, wgpu::ComputePipeline>>,
}

impl WgpuComputeContext {
    pub fn new(capacity_bytes: usize) -> Result<Self, ForgeError> {
        // Respect QUALIA_WGPU_BACKEND so the forge can use Vulkan (which exposes
        // cooperative matrix on NVIDIA) instead of the Windows DX12 default.
        let instance = create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .map_err(|error| ForgeError::GpuUnavailable(error.to_string()))?;
        let info = adapter.get_info();
        let available_features = adapter.features();
        // Request timestamp queries plus subgroup + cooperative-matrix support
        // when the adapter offers them (cooperative-matrix kernels need both).
        let mut wanted =
            wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::SUBGROUP | wgpu::Features::SHADER_F16;
        if crate::gpu_context::experimental_features_allowed() {
            wanted |= wgpu::Features::EXPERIMENTAL_COOPERATIVE_MATRIX
                | wgpu::Features::EXPERIMENTAL_RAY_QUERY;
        }
        let required_features = available_features & wanted;
        let timestamp_supported = required_features.contains(wgpu::Features::TIMESTAMP_QUERY);
        let limits = adapter.limits();
        // Populate intrinsic-capability flags from the adapter's real feature set
        // so the tuner can prune schedules that rely on absent hardware (plan §6).
        let mut constraints = AdapterConstraints::from_wgpu_limits(&limits);
        constraints.supports_subgroups = available_features.contains(wgpu::Features::SUBGROUP);
        constraints.supports_coopmat =
            available_features.contains(wgpu::Features::EXPERIMENTAL_COOPERATIVE_MATRIX);
        constraints.supports_rt_cores =
            available_features.contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY);
        // Warp/wavefront width by vendor: AMD wavefronts are 64, others 32.
        constraints.warp_size = if info.vendor == 0x1002 { 64 } else { 32 };
        // The ray-tracing acceleration-structure limits default to 0, which forbids
        // BLAS/TLAS creation even with EXPERIMENTAL_RAY_QUERY enabled. Raise them to
        // the adapter's supported values (a no-op on adapters that report 0).
        let required_limits = wgpu::Limits {
            max_blas_primitive_count: limits.max_blas_primitive_count,
            max_blas_geometry_count: limits.max_blas_geometry_count,
            max_tlas_instance_count: limits.max_tlas_instance_count,
            max_acceleration_structures_per_shader_stage: limits
                .max_acceleration_structures_per_shader_stage,
            ..wgpu::Limits::default()
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_features,
            required_limits,
            // The cooperative-matrix feature is gated behind wgpu's experimental
            // token. Only requested (above) when the adapter advertises it; the
            // token is harmless when no experimental feature is actually enabled.
            // Safety: we only use it for the cooperative-matrix matmul kernel.
            experimental_features: if required_features.intersects(
                wgpu::Features::EXPERIMENTAL_COOPERATIVE_MATRIX
                    | wgpu::Features::EXPERIMENTAL_RAY_QUERY,
            ) {
                unsafe { wgpu::ExperimentalFeatures::enabled() }
            } else {
                wgpu::ExperimentalFeatures::disabled()
            },
            ..Default::default()
        }))
        .map_err(|error| ForgeError::GpuUnavailable(error.to_string()))?;
        let timestamp_period_ns = if timestamp_supported {
            queue.get_timestamp_period()
        } else {
            0.0
        };

        // Determine topology (unified vs discrete)
        let topology = if info.device_type == wgpu::DeviceType::IntegratedGpu
            || info.device_type == wgpu::DeviceType::Cpu
        {
            MemoryTopology::Unified { zero_copy: true }
        } else {
            MemoryTopology::Discrete {
                staging_required: true,
            }
        };
        let memory_class = match topology {
            MemoryTopology::Unified { .. } => "unified",
            MemoryTopology::Discrete { .. } => "discrete",
        }
        .to_string();

        let adapter = AdapterIdentity {
            name: info.name,
            vendor: info.vendor,
            device: info.device,
            device_type: format!("{:?}", info.device_type),
            backend: format!("{:?}", info.backend),
            driver: info.driver,
            driver_info: info.driver_info,
        };
        let profile = HardwareProfile {
            adapter: adapter.clone(),
            constraints,
            memory_class,
            supports_timestamp_query: timestamp_supported,
            max_compute_workgroup_storage_size: limits.max_compute_workgroup_storage_size,
            max_storage_buffer_binding_size: limits.max_storage_buffer_binding_size,
            min_storage_buffer_offset_alignment: limits.min_storage_buffer_offset_alignment,
            min_uniform_buffer_offset_alignment: limits.min_uniform_buffer_offset_alignment,
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

        let out_slab = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forge-out-slab"),
            size: capacity_bytes as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let weight_slab = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forge-weight-slab"),
            size: capacity_bytes as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let timestamp_resources = timestamp_supported.then(|| TimestampResources::new(&device));

        Ok(Self {
            device,
            queue,
            adapter,
            constraints,
            profile,
            allocator,
            slab,
            out_slab,
            weight_slab,
            weight_cursor: 0,
            timestamp_supported,
            timestamp_period_ns,
            timestamp_resources,
            pipeline_cache: RefCell::new(HashMap::new()),
        })
    }

    /// Like [`new`] but tries the Vulkan backend first to find a cooperative-matrix-
    /// capable adapter. On Windows/DX12, `EXPERIMENTAL_COOPERATIVE_MATRIX` is not
    /// advertised, but the same NVIDIA GPU exposes `VK_KHR_cooperative_matrix` via
    /// the Vulkan driver. This constructor:
    ///
    /// 1. Creates a Vulkan-only instance (unless `QUALIA_WGPU_BACKEND` overrides).
    /// 2. Enumerates adapters, looking for one with `EXPERIMENTAL_COOPERATIVE_MATRIX`.
    /// 3. If found, builds the context on that adapter (un-gating coopmat).
    /// 4. If not found, falls back to [`new`] (which uses the default backend).
    ///
    /// This is the primary un-gating path for the HLSL WaveMatrix / WGSL coopmat
    /// tensor-core emitters on NVIDIA hardware where DX12 doesn't expose coopmat.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_for_coopmat(capacity_bytes: usize) -> Result<Self, ForgeError> {
        // Step 1: Try Vulkan instance.
        let vk_instance = create_instance_for_coopmat();

        // Step 2: Find a coopmat-capable adapter.
        if let Some(adapter) = find_coopmat_adapter(&vk_instance) {
            let info = adapter.get_info();
            log::info!(
                "forge|coopmat_adapter|{}|backend={:?}|vendor=0x{:04x}:0x{:04x}",
                info.name,
                info.backend,
                info.vendor,
                info.device
            );
            let available_features = adapter.features();
            let mut wanted = wgpu::Features::TIMESTAMP_QUERY
                | wgpu::Features::SUBGROUP
                | wgpu::Features::SHADER_F16;
            if crate::gpu_context::experimental_features_allowed() {
                wanted |= wgpu::Features::EXPERIMENTAL_COOPERATIVE_MATRIX
                    | wgpu::Features::EXPERIMENTAL_RAY_QUERY;
            }
            let required_features = available_features & wanted;
            let timestamp_supported = required_features.contains(wgpu::Features::TIMESTAMP_QUERY);
            let limits = adapter.limits();
            let mut constraints = AdapterConstraints::from_wgpu_limits(&limits);
            constraints.supports_subgroups = available_features.contains(wgpu::Features::SUBGROUP);
            constraints.supports_coopmat =
                available_features.contains(wgpu::Features::EXPERIMENTAL_COOPERATIVE_MATRIX);
            constraints.supports_rt_cores =
                available_features.contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY);
            constraints.warp_size = if info.vendor == 0x1002 { 64 } else { 32 };
            let required_limits = wgpu::Limits {
                max_blas_primitive_count: limits.max_blas_primitive_count,
                max_blas_geometry_count: limits.max_blas_geometry_count,
                max_tlas_instance_count: limits.max_tlas_instance_count,
                max_acceleration_structures_per_shader_stage: limits
                    .max_acceleration_structures_per_shader_stage,
                ..wgpu::Limits::default()
            };
            let (device, queue) =
                pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                    required_features,
                    required_limits,
                    experimental_features: if required_features.intersects(
                        wgpu::Features::EXPERIMENTAL_COOPERATIVE_MATRIX
                            | wgpu::Features::EXPERIMENTAL_RAY_QUERY,
                    ) {
                        unsafe { wgpu::ExperimentalFeatures::enabled() }
                    } else {
                        wgpu::ExperimentalFeatures::disabled()
                    },
                    ..Default::default()
                }))
                .map_err(|error| ForgeError::GpuUnavailable(error.to_string()))?;
            let timestamp_period_ns = if timestamp_supported {
                queue.get_timestamp_period()
            } else {
                0.0
            };
            let topology = if info.device_type == wgpu::DeviceType::IntegratedGpu
                || info.device_type == wgpu::DeviceType::Cpu
            {
                MemoryTopology::Unified { zero_copy: true }
            } else {
                MemoryTopology::Discrete {
                    staging_required: true,
                }
            };
            let memory_class = match topology {
                MemoryTopology::Unified { .. } => "unified",
                MemoryTopology::Discrete { .. } => "discrete",
            }
            .to_string();
            let adapter_id = AdapterIdentity {
                name: info.name,
                vendor: info.vendor,
                device: info.device,
                device_type: format!("{:?}", info.device_type),
                backend: format!("{:?}", info.backend),
                driver: info.driver,
                driver_info: info.driver_info,
            };
            let profile = HardwareProfile {
                adapter: adapter_id.clone(),
                constraints,
                memory_class,
                supports_timestamp_query: timestamp_supported,
                max_compute_workgroup_storage_size: limits.max_compute_workgroup_storage_size,
                max_storage_buffer_binding_size: limits.max_storage_buffer_binding_size,
                min_storage_buffer_offset_alignment: limits.min_storage_buffer_offset_alignment,
                min_uniform_buffer_offset_alignment: limits.min_uniform_buffer_offset_alignment,
            };
            let allocator = QualiaSlabAllocator::new(topology, capacity_bytes);
            let slab = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("forge-slab-coopmat"),
                size: capacity_bytes as u64,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let out_slab = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("forge-out-slab-coopmat"),
                size: capacity_bytes as u64,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let weight_slab = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("forge-weight-slab-coopmat"),
                size: capacity_bytes as u64,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let timestamp_resources = timestamp_supported.then(|| TimestampResources::new(&device));
            return Ok(Self {
                device,
                queue,
                adapter: adapter_id,
                constraints,
                profile,
                allocator,
                slab,
                out_slab,
                weight_slab,
                weight_cursor: 0,
                timestamp_supported,
                timestamp_period_ns,
                timestamp_resources,
                pipeline_cache: RefCell::new(HashMap::new()),
            });
        }

        // Step 3: No coopmat adapter found — fall back to the default backend.
        log::info!("forge|coopmat_adapter|none_found|falling_back_to_default");
        Self::new(capacity_bytes)
    }

    /// Build a forge context on an **already-existing** `wgpu::Device` + `Queue` (e.g. the
    /// process-wide [`crate::gpu_context::shared_gpu`]) instead of requesting a *second* adapter
    /// and device the way [`Self::new`] does. wgpu `Device`/`Queue` are cheap `Arc` clones, so the
    /// forge then runs on the **same** device that owns the resident LLM weights + KV cache — the
    /// device-unification keystone for running decode on the forge (LLM-on-forge plan, Phase 1a).
    ///
    /// Adapter identity / constraints / hardware profile are reconstructed from the **live**
    /// `device.limits()` + `device.features()` plus the caller's [`GpuAdapterCaps`] snapshot,
    /// because the original `wgpu::Adapter` is consumed at shared-gpu init and not retained.
    ///
    /// Honest boundary: this inherits the *host* device's negotiated features and limits verbatim.
    /// In particular, if the shared device was created without the ray-tracing acceleration-structure
    /// limits raised (as `shared_gpu` currently does), RT-core Neighbor cannot create BLAS/TLAS on
    /// this context even when `supports_rt_cores` is true — `from_device` does not silently widen the
    /// host device. The decode path (matmul/elementwise/reduce) needs none of that.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_device(
        device: wgpu::Device,
        queue: wgpu::Queue,
        caps: &GpuAdapterCaps,
        capacity_bytes: usize,
    ) -> Result<Self, ForgeError> {
        let features = device.features();
        let limits = device.limits();
        let timestamp_supported = features.contains(wgpu::Features::TIMESTAMP_QUERY);

        // Mirror `new()`'s capability derivation, but from the live device + caps snapshot.
        let mut constraints = AdapterConstraints::from_wgpu_limits(&limits);
        constraints.supports_subgroups = features.contains(wgpu::Features::SUBGROUP);
        constraints.supports_coopmat =
            features.contains(wgpu::Features::EXPERIMENTAL_COOPERATIVE_MATRIX);
        constraints.supports_rt_cores = features.contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY);
        // AMD wavefronts are 64-wide; others 32. Vendor id 0x1002 = AMD.
        constraints.warp_size = if caps.vendor == 0x1002 { 64 } else { 32 };

        let topology = if matches!(
            caps.device_type,
            wgpu::DeviceType::IntegratedGpu | wgpu::DeviceType::Cpu
        ) {
            MemoryTopology::Unified { zero_copy: true }
        } else {
            MemoryTopology::Discrete {
                staging_required: true,
            }
        };
        let memory_class = match topology {
            MemoryTopology::Unified { .. } => "unified",
            MemoryTopology::Discrete { .. } => "discrete",
        }
        .to_string();

        let adapter = AdapterIdentity {
            name: caps.name.clone(),
            vendor: caps.vendor,
            device: caps.device,
            device_type: format!("{:?}", caps.device_type),
            backend: format!("{:?}", caps.backend),
            driver: caps.driver.clone(),
            driver_info: caps.driver_info.clone(),
        };
        let timestamp_period_ns = if timestamp_supported {
            queue.get_timestamp_period()
        } else {
            0.0
        };
        let profile = HardwareProfile {
            adapter: adapter.clone(),
            constraints,
            memory_class,
            supports_timestamp_query: timestamp_supported,
            max_compute_workgroup_storage_size: limits.max_compute_workgroup_storage_size,
            max_storage_buffer_binding_size: limits.max_storage_buffer_binding_size,
            min_storage_buffer_offset_alignment: limits.min_storage_buffer_offset_alignment,
            min_uniform_buffer_offset_alignment: limits.min_uniform_buffer_offset_alignment,
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
        let out_slab = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forge-out-slab"),
            size: capacity_bytes as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let weight_slab = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forge-weight-slab"),
            size: capacity_bytes as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let timestamp_resources = timestamp_supported.then(|| TimestampResources::new(&device));

        Ok(Self {
            device,
            queue,
            adapter,
            constraints,
            profile,
            allocator,
            slab,
            out_slab,
            weight_slab,
            weight_cursor: 0,
            timestamp_supported,
            timestamp_period_ns,
            timestamp_resources,
            pipeline_cache: RefCell::new(HashMap::new()),
        })
    }

    /// The physical buffer backing a view, chosen by its binding usage.
    fn slab_for(&self, usage: BindingUsage) -> &wgpu::Buffer {
        match usage {
            BindingUsage::StorageReadWrite => &self.out_slab,
            BindingUsage::StorageReadResident => &self.weight_slab,
            BindingUsage::StorageRead | BindingUsage::Uniform => &self.slab,
        }
    }

    /// Allocate a transient slab sub-range and upload `data` into it.
    ///
    /// # Topology note (honest scope, plan §2)
    ///
    /// This upload uses `queue.write_buffer` **uniformly on every topology**
    /// (unified and discrete alike); readback in [`Self::read_buffer_f32`]
    /// likewise uses `copy_buffer_to_buffer` uniformly. The
    /// `MemoryTopology::{Unified, Discrete}` classification on the allocator is
    /// *recorded but not yet acted upon here*: the plan-§2 differentiated paths
    /// (zero-copy persistent-mapped slabs for unified memory; a pinned staging ring
    /// with async `copy_buffer` for discrete PCIe) are NOT implemented. The current
    /// uniform path is correct on both topologies but unoptimised; the unified
    /// zero-copy benefit cannot be measured on this discrete-only host (RTX A2000),
    /// so it is left as documented future work rather than shipped unverified. See
    /// [`MemoryTopology`] for the full rationale.
    pub fn allocate_and_write(
        &mut self,
        data: &[u8],
        binding: u32,
        group: u32,
        usage: BindingUsage,
    ) -> Result<BufferView, ForgeError> {
        let view = self
            .allocator
            .allocate_transient(data.len(), binding, group, usage)?;
        if !data.is_empty() {
            let slab = self.slab_for(usage);
            self.queue
                .write_buffer(slab, view.offset as wgpu::BufferAddress, data);
        }
        Ok(view)
    }

    /// Bump-allocate `data` into the **persistent weight region** (`weight_slab`) and upload it
    /// once, returning a [`BufferView`] tagged [`BindingUsage::StorageReadResident`]. Unlike
    /// [`Self::allocate_and_write`] (transient ring), this view **survives**
    /// [`Self::clear_transient_allocations`], so a decode layer's projection / FFN matrices are
    /// uploaded a single time and referenced by offset across every token's `run` — eliminating
    /// the per-call weight re-upload. Offsets are 256-aligned for direct bind-group use.
    pub fn allocate_weight(
        &mut self,
        data: &[u8],
        binding: u32,
        group: u32,
    ) -> Result<BufferView, ForgeError> {
        let offset =
            self.weight_cursor.div_ceil(DEFAULT_BINDING_ALIGNMENT) * DEFAULT_BINDING_ALIGNMENT;
        let end = offset + data.len();
        let cap = self.weight_slab.size() as usize;
        if end > cap {
            return Err(ForgeError::GpuValidation(format!(
                "weight region overflow: need {end} bytes but weight slab is {cap} (raise capacity)"
            )));
        }
        if !data.is_empty() {
            self.queue
                .write_buffer(&self.weight_slab, offset as wgpu::BufferAddress, data);
        }
        self.weight_cursor = end;
        Ok(BufferView {
            offset,
            length_bytes: data.len(),
            binding,
            group,
            usage: BindingUsage::StorageReadResident,
        })
    }

    /// Reset the persistent weight region so it can be reused for a different model/layer set.
    /// Any [`BufferView`]s previously returned by [`Self::allocate_weight`] become stale — drop
    /// the corresponding handles and re-load. (Weights are write-once; no per-tensor free.)
    pub fn clear_weights(&mut self) {
        self.weight_cursor = 0;
    }

    /// Bytes currently consumed in the persistent weight region (for tests / introspection).
    pub fn resident_weight_bytes(&self) -> usize {
        self.weight_cursor
    }

    pub fn allocate_transient(
        &mut self,
        size_bytes: usize,
        binding: u32,
        group: u32,
        usage: BindingUsage,
    ) -> Result<BufferView, ForgeError> {
        self.allocator
            .allocate_transient(size_bytes, binding, group, usage)
    }

    pub fn advance_read_head(&mut self, offset: usize) {
        self.allocator.advance_read_head(offset);
    }

    pub fn clear_transient_allocations(&mut self) {
        self.allocator.clear();
    }

    /// Builds a bottom-level (BLAS) + top-level (TLAS) acceleration structure for a
    /// triangle soup and returns both, ready to bind to a ray-query shader. `vertices`
    /// is a flat list of `f32` triples (3 per vertex, 3 vertices per triangle),
    /// row-major. The BLAS geometry is marked `OPAQUE` (required — naga's ray-query
    /// has no candidate/any-hit path, so non-opaque geometry yields no committed hits),
    /// and the single TLAS instance uses the identity transform. Both structures are
    /// built and the queue drained before returning. Requires the adapter to support
    /// (and the device to have enabled) `EXPERIMENTAL_RAY_QUERY`.
    ///
    /// The returned `Blas` must be kept alive alongside the `Tlas` for the lifetime of
    /// any bind group referencing the TLAS (the `TlasInstance` borrows the BLAS).
    pub fn build_triangle_scene(
        &self,
        vertices: &[f32],
    ) -> Result<(wgpu::Blas, wgpu::Tlas), ForgeError> {
        if !self.constraints.supports_rt_cores {
            return Err(ForgeError::GpuUnavailable(
                "adapter lacks ray-query (RT) support".to_string(),
            ));
        }
        if vertices.is_empty() || vertices.len() % 9 != 0 {
            return Err(ForgeError::GpuValidation(format!(
                "triangle scene needs a non-empty multiple of 9 floats (3 verts x xyz); got {}",
                vertices.len()
            )));
        }
        let vertex_count = (vertices.len() / 3) as u32;

        let vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forge-blas-vertices"),
            size: (vertices.len() * size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::BLAS_INPUT | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(vertices));

        let size_desc = wgpu::BlasTriangleGeometrySizeDescriptor {
            vertex_format: wgpu::VertexFormat::Float32x3,
            vertex_count,
            index_format: None,
            index_count: None,
            flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
        };
        let blas = self.device.create_blas(
            &wgpu::CreateBlasDescriptor {
                label: Some("forge-blas"),
                flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                update_mode: wgpu::AccelerationStructureUpdateMode::Build,
            },
            wgpu::BlasGeometrySizeDescriptors::Triangles {
                descriptors: vec![size_desc.clone()],
            },
        );

        let mut tlas = self.device.create_tlas(&wgpu::CreateTlasDescriptor {
            label: Some("forge-tlas"),
            max_instances: 1,
            flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
            update_mode: wgpu::AccelerationStructureUpdateMode::Build,
        });
        // 3x4 row-major identity (scene vertices are already in world space).
        let identity: [f32; 12] = [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ];
        tlas[0] = Some(wgpu::TlasInstance::new(&blas, identity, 0, 0xFF));

        let geometry = wgpu::BlasTriangleGeometry {
            size: &size_desc,
            vertex_buffer: &vertex_buffer,
            first_vertex: 0,
            vertex_stride: (3 * size_of::<f32>()) as wgpu::BufferAddress,
            index_buffer: None,
            first_index: None,
            transform_buffer: None,
            transform_buffer_offset: None,
        };
        let entry = wgpu::BlasBuildEntry {
            blas: &blas,
            geometry: wgpu::BlasGeometries::TriangleGeometries(vec![geometry]),
        };
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("forge-build-accel"),
            });
        encoder.build_acceleration_structures(std::iter::once(&entry), std::iter::once(&tlas));
        self.queue.submit(Some(encoder.finish()));
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| {
                ForgeError::DeviceLost(format!("device poll failed building accel: {e:?}"))
            })?;

        Ok((blas, tlas))
    }

    /// Compile a WGSL compute pipeline and return the **owned** `wgpu::ComputePipeline`
    /// (no borrow of `self`), wrapped in a validation error scope. This is the building
    /// block the multi-node graph executor uses to compile every node's kernel up front
    /// before recording them into a single command encoder ([`Self::submit_graph`]).
    /// [`WgpuPipeline::compile`] delegates here.
    pub fn compile_pipeline(
        &self,
        source: &str,
        entry_point: &str,
    ) -> Result<wgpu::ComputePipeline, ForgeError> {
        let error_scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("qualia-wgsl-forge"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
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
        Ok(pipeline)
    }

    /// Like [`compile_pipeline`] but accepts pre-compiled SPIR-V bytes instead of
    /// WGSL source. This is the execution bridge for native shader profiles that
    /// compile to SPIR-V (notably HLSL via DXC `–spirv`): the forge emits HLSL,
    /// DXC produces a SPIR-V binary, and this method feeds it into the same wgpu
    /// pipeline (bind groups, slab, dispatch — all unchanged).
    pub fn compile_pipeline_spirv(
        &self,
        spirv: &[u8],
        entry_point: &str,
    ) -> Result<wgpu::ComputePipeline, ForgeError> {
        let error_scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let spirv_words: &[u32] = bytemuck::cast_slice(spirv);
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("qualia-forge-spirv"),
                source: wgpu::ShaderSource::SpirV(std::borrow::Cow::Borrowed(spirv_words)),
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("qualia-forge-spirv-pipeline"),
                layout: None,
                module: &shader,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                cache: None,
            });
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            return Err(ForgeError::GpuValidation(error.to_string()));
        }
        Ok(pipeline)
    }

    /// [`compile_pipeline`](Self::compile_pipeline) with a process-lifetime cache keyed by
    /// `entry\0source` — the same `(source, entry)` returns the previously-built pipeline
    /// (a cheap `Arc`-clone) instead of recompiling. This is what makes a re-run of a fixed
    /// graph (e.g. one decode block per generated token, via a held [`ForgeGraphExecutor`])
    /// pay shader compilation **once**, not per call. The graph executor records its nodes
    /// through this path; one-shot callers see a cold cache (built + dropped with the context).
    pub fn compile_pipeline_cached(
        &self,
        source: &str,
        entry_point: &str,
    ) -> Result<wgpu::ComputePipeline, ForgeError> {
        let key = format!("{entry_point}\u{0}{source}");
        if let Some(pipeline) = self.pipeline_cache.borrow().get(&key) {
            return Ok(pipeline.clone());
        }
        let pipeline = self.compile_pipeline(source, entry_point)?;
        self.pipeline_cache
            .borrow_mut()
            .insert(key, pipeline.clone());
        Ok(pipeline)
    }

    /// Number of distinct pipelines currently cached (for tests / introspection).
    pub fn cached_pipeline_count(&self) -> usize {
        self.pipeline_cache.borrow().len()
    }

    /// Build a bind group binding each [`BufferView`] at its `binding` slot, choosing the
    /// physical slab per the view's usage ([`Self::slab_for`]). Shared by the per-node
    /// [`WgpuPipeline::dispatch`] path and the deferred-submit graph path.
    pub fn create_compute_bind_group(
        &self,
        pipeline: &wgpu::ComputePipeline,
        buffers: &[BufferView],
    ) -> wgpu::BindGroup {
        let mut entries = Vec::with_capacity(buffers.len());
        for view in buffers {
            let size = wgpu::BufferSize::new(view.length_bytes as u64);
            entries.push(wgpu::BindGroupEntry {
                binding: view.binding,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: self.slab_for(view.usage),
                    offset: view.offset as wgpu::BufferAddress,
                    size,
                }),
            });
        }
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("forge-bind-group"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        })
    }

    /// Record **all** of a graph's node dispatches — and the GPU→GPU hand-off copies
    /// between them — into ONE [`wgpu::CommandEncoder`] and submit it **once**, instead of
    /// one `queue.submit()` per node. This is the single-encoder deferred-submit fusion
    /// (plan §8.1 "Option B"): within one command buffer wgpu preserves command order and
    /// inserts the necessary buffer hazard barriers, so a producer's compute pass, its
    /// `copy_buffer_to_buffer` hand-off, and the consumer's dispatch are correctly ordered
    /// with no host round-trip and no per-node submit latency. The caller (the executor)
    /// has already encoded each node's data dependencies in `passes` (insertion/topological
    /// order) and built each bind group, so this loop is pure recording. Blocks on device
    /// completion and surfaces any validation error.
    pub fn submit_graph(&self, passes: &[GraphPass]) -> Result<(), ForgeError> {
        let error_scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("forge-graph-encoder"),
            });
        for pass in passes {
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("forge-graph-pass"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&pass.pipeline);
                cpass.set_bind_group(0, &pass.bind_group, &[]);
                cpass.dispatch_workgroups(pass.workgroups, 1, 1);
            }
            // GPU→GPU hand-off: copy this node's read_write output into the fresh
            // read-slab buffer a downstream node will bind read-only. Recorded in the
            // SAME encoder, so it is ordered after the pass that produced it.
            if let Some((src, dst)) = &pass.copy {
                let len = src.length_bytes.min(dst.length_bytes) as u64;
                if len > 0 {
                    encoder.copy_buffer_to_buffer(
                        self.slab_for(src.usage),
                        src.offset as u64,
                        self.slab_for(dst.usage),
                        dst.offset as u64,
                        len,
                    );
                }
            }
        }
        self.queue.submit(Some(encoder.finish()));
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| {
                ForgeError::DeviceLost(format!("device poll failed submitting graph: {e:?}"))
            })?;
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            return Err(ForgeError::GpuValidation(error.to_string()));
        }
        Ok(())
    }

    /// Copy `src`'s bytes to `dst` on the device (GPU→GPU, no host readback), honouring
    /// each view's slab. Used by the multi-node graph executor to move a node's output out
    /// of the read_write slab into the read slab, so a downstream node can bind it as a
    /// read-only input without aliasing its own read_write output (wgpu forbids the same
    /// buffer being bound read-write and read-only within one dispatch). Submits on the
    /// shared queue, so it is ordered before any later dispatch that reads `dst`.
    pub fn copy_view(&self, src: &BufferView, dst: &BufferView) -> Result<(), ForgeError> {
        let len = src.length_bytes.min(dst.length_bytes) as u64;
        if len == 0 {
            return Ok(());
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("forge-view-copy"),
            });
        encoder.copy_buffer_to_buffer(
            self.slab_for(src.usage),
            src.offset as u64,
            self.slab_for(dst.usage),
            dst.offset as u64,
            len,
        );
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    pub fn read_buffer_f32(&self, view: &BufferView) -> Result<Vec<f32>, ForgeError> {
        let size = view.length_bytes as u64;
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forge-output-staging"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("forge-output-copy"),
            });
        encoder.copy_buffer_to_buffer(
            self.slab_for(view.usage),
            view.offset as u64,
            &staging,
            0,
            size,
        );
        self.queue.submit(Some(encoder.finish()));
        let bytes = map_read(&self.device, &staging)?;

        let elements = view.length_bytes / size_of::<f32>();
        let output = bytemuck::cast_slice::<u8, f32>(&bytes)[..elements].to_vec();
        drop(bytes);
        staging.unmap();
        Ok(output)
    }
}

impl OracleContext for WgpuComputeContext {
    fn allocate_and_write(
        &mut self,
        data: &[u8],
        binding: u32,
        group: u32,
        usage: BindingUsage,
    ) -> Result<BufferView, ForgeError> {
        WgpuComputeContext::allocate_and_write(self, data, binding, group, usage)
    }

    fn allocate_transient(
        &mut self,
        size_bytes: usize,
        binding: u32,
        group: u32,
        usage: BindingUsage,
    ) -> Result<BufferView, ForgeError> {
        WgpuComputeContext::allocate_transient(self, size_bytes, binding, group, usage)
    }

    fn read_buffer_f32(&self, view: &BufferView) -> Result<Vec<f32>, ForgeError> {
        WgpuComputeContext::read_buffer_f32(self, view)
    }

    fn clear_transient_allocations(&mut self) {
        WgpuComputeContext::clear_transient_allocations(self);
    }

    fn adapter(&self) -> &AdapterIdentity {
        &self.adapter
    }

    fn constraints(&self) -> &AdapterConstraints {
        &self.constraints
    }

    fn timestamp_supported(&self) -> bool {
        self.timestamp_supported
    }

    /// Emit the kernel's WGSL, compile it, then run the warmup + timed-sample
    /// dispatch loop — byte-for-byte the loop the wgpu oracle evaluators ran inline
    /// (warmups untimed, then `samples` timed dispatches via [`QualiaCompute::dispatch`]).
    fn run_kernel(
        &mut self,
        kernel: &KernelSpec,
        schedule: &Schedule,
        buffers: &[BufferView],
        element_count: usize,
        warmups: usize,
        samples: usize,
    ) -> Result<Vec<u64>, ForgeError> {
        let generated = emit_shader(kernel, *schedule, TargetBackend::Wgsl)?;
        let pipeline = WgpuPipeline::compile(self, &generated.source, &kernel.entry_point)?;

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

/// One recorded graph node, ready for [`WgpuComputeContext::submit_graph`] to play into a
/// shared command encoder: the compiled pipeline, its bind group, the workgroup count, and
/// the optional GPU→GPU hand-off copy (`src` in the read_write slab → `dst` in the read
/// slab) emitted after the node's compute pass. The pipeline and bind group are owned so the
/// executor can build them all up front and submit the whole graph in one go.
pub struct GraphPass {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group: wgpu::BindGroup,
    pub workgroups: u32,
    pub copy: Option<(BufferView, BufferView)>,
}

pub struct WgpuPipeline<'a> {
    context: &'a WgpuComputeContext,
    pipeline: wgpu::ComputePipeline,
}

impl<'a> WgpuPipeline<'a> {
    pub fn compile(
        context: &'a WgpuComputeContext,
        source: &str,
        entry_point: &str,
    ) -> Result<Self, ForgeError> {
        let pipeline = context.compile_pipeline(source, entry_point)?;
        Ok(Self { context, pipeline })
    }

    /// Like [`compile`] but accepts pre-compiled SPIR-V bytes (from HLSL→DXC).
    pub fn compile_spirv(
        context: &'a WgpuComputeContext,
        spirv: &[u8],
        entry_point: &str,
    ) -> Result<Self, ForgeError> {
        let pipeline = context.compile_pipeline_spirv(spirv, entry_point)?;
        Ok(Self { context, pipeline })
    }

    /// Compile pre-emitted MSL source through wgpu's Metal backend.
    ///
    /// wgpu does not expose `ShaderSource::Msl` — on macOS, wgpu transpiles
    /// WGSL→MSL internally via naga. For pre-emitted MSL, the caller should
    /// use a native Metal path (e.g. `metal-rs`). This method exists so the
    /// runtime can attempt MSL compilation on macOS; on other platforms it
    /// returns an error and the runtime falls back to WGSL.
    pub fn compile_msl(
        context: &'a WgpuComputeContext,
        _source: &str,
        _entry_point: &str,
    ) -> Result<Self, ForgeError> {
        // wgpu's ShaderSource enum does not have an Msl variant.
        // On macOS, the practical path is to transpile MSL→SPIR-V via
        // SPIRV-Cross and use compile_spirv, or use metal-rs directly.
        // For now, return an error so the runtime falls back to WGSL.
        let _ = context;
        Err(ForgeError::Emission(
            "MSL native compilation requires a metal-rs bridge (not yet implemented). \
             Falling back to WGSL."
                .to_string(),
        ))
    }

    /// Dispatch a ray-query kernel, binding `tlas` as the `acceleration_structure`
    /// at binding 0 and the supplied buffer views (rays at binding 1, hits at
    /// binding 2) at their own bindings. The generic [`QualiaCompute::dispatch`]
    /// only binds buffers, so this is the dedicated path for the acceleration-
    /// structure binding. Returns wall-clock nanoseconds (ray-query passes skip the
    /// timestamp path). The caller must keep the TLAS (and its BLAS) alive.
    pub fn dispatch_rayprobe(
        &self,
        tlas: &wgpu::Tlas,
        buffers: &[BufferView],
        schedule: &Schedule,
        element_count: usize,
    ) -> Result<u64, ForgeError> {
        let mut entries = Vec::with_capacity(buffers.len() + 1);
        entries.push(wgpu::BindGroupEntry {
            binding: 0,
            resource: tlas.as_binding(),
        });
        for view in buffers {
            let size = wgpu::BufferSize::new(view.length_bytes as u64);
            entries.push(wgpu::BindGroupEntry {
                binding: view.binding,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: self.context.slab_for(view.usage),
                    offset: view.offset as wgpu::BufferAddress,
                    size,
                }),
            });
        }
        let bind_group = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("forge-rayprobe-bind-group"),
                layout: &self.pipeline.get_bind_group_layout(0),
                entries: &entries,
            });
        let dispatch_x = schedule.dispatch_workgroups(element_count);

        let started = Instant::now();
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("forge-rayprobe-dispatch"),
                });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("forge-rayprobe-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(dispatch_x, 1, 1);
        }
        self.context.queue.submit(Some(encoder.finish()));
        let _ = self
            .context
            .device
            .poll(wgpu::PollType::wait_indefinitely());
        Ok(started.elapsed().as_nanos().min(u64::MAX as u128) as u64)
    }
}

impl<'a> QualiaCompute for WgpuPipeline<'a> {
    fn dispatch(
        &self,
        buffers: &[BufferView],
        schedule: &Schedule,
        element_count: usize,
    ) -> Result<u64, ForgeError> {
        let bind_group = self
            .context
            .create_compute_bind_group(&self.pipeline, buffers);
        let dispatch_x = schedule.dispatch_workgroups(element_count);

        let started = Instant::now();
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("forge-dispatch"),
                });
        {
            let timestamp_writes = self.context.timestamp_resources.as_ref().map(|resources| {
                wgpu::ComputePassTimestampWrites {
                    query_set: &resources.query_set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                }
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
            self.context
                .device
                .poll(wgpu::PollType::wait_indefinitely())
                .map_err(|e| ForgeError::DeviceLost(format!("device poll failed: {e:?}")))?;
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
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| ForgeError::DeviceLost(format!("device poll failed during map: {e:?}")))?;
    receiver
        .recv()
        .map_err(|error| ForgeError::GpuValidation(error.to_string()))?
        .map_err(|error| ForgeError::GpuValidation(error.to_string()))?;
    // wgpu 30: get_mapped_range() returns Result — propagate as ForgeError.
    slice
        .get_mapped_range()
        .map_err(|e| ForgeError::GpuValidation(format!("map_range failed: {e:?}")))
}
