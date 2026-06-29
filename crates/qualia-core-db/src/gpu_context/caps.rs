//! Native wgpu adapter capability reporting.
//!
//! This module is intentionally diagnostic/policy-facing rather than hot-path code:
//! it records what the selected adapter actually exposes so benchmark rows, logs,
//! and future feature negotiation can agree on the same facts.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GpuFeatureCaps {
    pub timestamp_query: bool,
    pub timestamp_query_inside_passes: bool,
    pub pipeline_statistics_query: bool,
    pub pipeline_cache: bool,
    pub shader_f16: bool,
    pub shader_int64: bool,
    pub subgroup: bool,
    pub subgroup_barrier: bool,
    pub cooperative_matrix: bool,
    pub ray_query: bool,
}

impl GpuFeatureCaps {
    pub fn from_features(features: wgpu::Features) -> Self {
        Self {
            timestamp_query: features.contains(wgpu::Features::TIMESTAMP_QUERY),
            timestamp_query_inside_passes: features
                .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES),
            pipeline_statistics_query: features.contains(wgpu::Features::PIPELINE_STATISTICS_QUERY),
            pipeline_cache: features.contains(wgpu::Features::PIPELINE_CACHE),
            shader_f16: features.contains(wgpu::Features::SHADER_F16),
            shader_int64: features.contains(wgpu::Features::SHADER_INT64),
            subgroup: features.contains(wgpu::Features::SUBGROUP),
            subgroup_barrier: features.contains(wgpu::Features::SUBGROUP_BARRIER),
            cooperative_matrix: features.contains(wgpu::Features::EXPERIMENTAL_COOPERATIVE_MATRIX),
            ray_query: features.contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY),
        }
    }

    pub fn compact_flags(&self) -> String {
        let mut out = String::with_capacity(96);
        push_flag(&mut out, "ts", self.timestamp_query);
        push_flag(&mut out, "ts-pass", self.timestamp_query_inside_passes);
        push_flag(&mut out, "stats", self.pipeline_statistics_query);
        push_flag(&mut out, "cache", self.pipeline_cache);
        push_flag(&mut out, "f16", self.shader_f16);
        push_flag(&mut out, "i64", self.shader_int64);
        push_flag(&mut out, "subgroup", self.subgroup);
        push_flag(&mut out, "subgroup-barrier", self.subgroup_barrier);
        push_flag(&mut out, "coop-matrix", self.cooperative_matrix);
        push_flag(&mut out, "ray-query", self.ray_query);
        out
    }
}

pub fn requested_native_llm_features(available: wgpu::Features) -> wgpu::Features {
    let mut desired = wgpu::Features::TIMESTAMP_QUERY
        | wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES
        | wgpu::Features::PIPELINE_STATISTICS_QUERY
        | wgpu::Features::PIPELINE_CACHE
        | wgpu::Features::SHADER_F16
        | wgpu::Features::SUBGROUP
        | wgpu::Features::SUBGROUP_BARRIER;
    if experimental_features_allowed() {
        desired |= wgpu::Features::EXPERIMENTAL_COOPERATIVE_MATRIX;
    }
    available & desired
}

pub fn experimental_features_allowed() -> bool {
    matches!(
        std::env::var("QUALIA_WGPU_EXPERIMENTAL_FEATURES")
            .ok()
            .as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GpuLimitCaps {
    pub max_buffer_size: u64,
    pub max_storage_buffer_binding_size: u64,
    pub max_compute_workgroup_storage_size: u32,
    pub max_compute_invocations_per_workgroup: u32,
    pub max_compute_workgroup_size_x: u32,
    pub max_compute_workgroups_per_dimension: u32,
}

impl GpuLimitCaps {
    pub fn from_limits(limits: &wgpu::Limits) -> Self {
        Self {
            max_buffer_size: limits.max_buffer_size,
            max_storage_buffer_binding_size: limits.max_storage_buffer_binding_size,
            max_compute_workgroup_storage_size: limits.max_compute_workgroup_storage_size,
            max_compute_invocations_per_workgroup: limits.max_compute_invocations_per_workgroup,
            max_compute_workgroup_size_x: limits.max_compute_workgroup_size_x,
            max_compute_workgroups_per_dimension: limits.max_compute_workgroups_per_dimension,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GpuAdapterCaps {
    pub name: String,
    pub backend: wgpu::Backend,
    pub device_type: wgpu::DeviceType,
    pub vendor: u32,
    pub device: u32,
    pub driver: String,
    pub driver_info: String,
    pub subgroup_min_size: u32,
    pub subgroup_max_size: u32,
    pub cooperative_matrix_tile_count: usize,
    pub features: GpuFeatureCaps,
    pub limits: GpuLimitCaps,
}

impl GpuAdapterCaps {
    pub fn from_adapter(adapter: &wgpu::Adapter) -> Self {
        let info = adapter.get_info();
        let features = adapter.features();
        let limits = adapter.limits();
        Self {
            name: info.name,
            backend: info.backend,
            device_type: info.device_type,
            vendor: info.vendor,
            device: info.device,
            driver: info.driver,
            driver_info: info.driver_info,
            subgroup_min_size: info.subgroup_min_size,
            subgroup_max_size: info.subgroup_max_size,
            cooperative_matrix_tile_count: adapter.cooperative_matrix_properties().len(),
            features: GpuFeatureCaps::from_features(features),
            limits: GpuLimitCaps::from_limits(&limits),
        }
    }

    #[inline]
    pub fn is_integrated_gpu(&self) -> bool {
        matches!(self.device_type, wgpu::DeviceType::IntegratedGpu)
    }

    #[inline]
    pub fn backend_label(&self) -> &'static str {
        match self.backend {
            wgpu::Backend::Noop => "noop",
            wgpu::Backend::Vulkan => "vulkan",
            wgpu::Backend::Metal => "metal",
            wgpu::Backend::Dx12 => "dx12",
            wgpu::Backend::Gl => "gl",
            wgpu::Backend::BrowserWebGpu => "browser-webgpu",
        }
    }

    #[inline]
    pub fn device_type_label(&self) -> &'static str {
        match self.device_type {
            wgpu::DeviceType::Other => "other",
            wgpu::DeviceType::IntegratedGpu => "integrated",
            wgpu::DeviceType::DiscreteGpu => "discrete",
            wgpu::DeviceType::VirtualGpu => "virtual",
            wgpu::DeviceType::Cpu => "cpu",
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "{} | backend={} | type={} | vendor=0x{:04x} | device=0x{:04x} | driver={} {}",
            self.name,
            self.backend_label(),
            self.device_type_label(),
            self.vendor,
            self.device,
            self.driver,
            self.driver_info
        )
    }

    pub fn llm_feature_line(&self) -> String {
        format!(
            "features=[{}] subgroup={}..{} coop_tiles={} max_storage_binding={}MiB max_buffer={}MiB",
            self.features.compact_flags(),
            self.subgroup_min_size,
            self.subgroup_max_size,
            self.cooperative_matrix_tile_count,
            self.limits.max_storage_buffer_binding_size / (1024 * 1024),
            self.limits.max_buffer_size / (1024 * 1024)
        )
    }
}

fn push_flag(out: &mut String, label: &str, enabled: bool) {
    if !out.is_empty() {
        out.push(' ');
    }
    out.push_str(label);
    out.push('=');
    out.push_str(if enabled { "1" } else { "0" });
}

// ── Inference-pipeline (GPU backend) selection — the "which pipeline for this machine" checker ──

/// An explicit GPU-backend override for the inference device, from `QUALIA_WGPU_BACKEND`
/// (`vulkan` | `dx12` | `metal` | `gl` | `primary` | `all`). `None` ⇒ use wgpu's default selection
/// (which still honors wgpu's own `WGPU_BACKEND`). This is what lets a machine be pinned to the
/// **vendor-neutral Vulkan** path (AMD/Intel/NVIDIA/ARM/Linux) rather than whatever wgpu picks.
#[cfg(not(target_arch = "wasm32"))]
pub fn qualia_backend_override() -> Option<wgpu::Backends> {
    let raw = std::env::var("QUALIA_WGPU_BACKEND").ok()?;
    let v = raw.trim().to_ascii_lowercase();
    Some(match v.as_str() {
        "vulkan" | "vk" => wgpu::Backends::VULKAN,
        "dx12" | "d3d12" | "directx12" => wgpu::Backends::DX12,
        "metal" | "mtl" => wgpu::Backends::METAL,
        "gl" | "opengl" | "gles" => wgpu::Backends::GL,
        "primary" => wgpu::Backends::PRIMARY,
        "all" => wgpu::Backends::all(),
        other => {
            log::warn!("QUALIA_WGPU_BACKEND='{other}' unrecognized — using wgpu default selection");
            return None;
        }
    })
}

/// Capability-aware recommendation for which GPU backend this machine *should* run inference on
/// (advisory; surfaced by the doctor/setup checker). Prefers the **portable, vendor-neutral**
/// path so the build is not silently locked to Windows (DX12) or NVIDIA (CUDA). Reactive to the
/// adapter actually in hand; a full enumerate-all-backends-and-pick is the next layer.
#[cfg(not(target_arch = "wasm32"))]
pub fn recommend_inference_backend(caps: &GpuAdapterCaps) -> &'static str {
    match caps.backend {
        wgpu::Backend::Vulkan => "vulkan — vendor-neutral & portable (recommended)",
        wgpu::Backend::Metal => "metal — Apple-native (recommended on macOS)",
        wgpu::Backend::Dx12 => {
            "dx12 — Windows-native; set QUALIA_WGPU_BACKEND=vulkan for the portable path"
        }
        wgpu::Backend::Gl => "gl — compatibility fallback, limited compute (last resort)",
        _ => "noop/unknown — no GPU compute path",
    }
}
