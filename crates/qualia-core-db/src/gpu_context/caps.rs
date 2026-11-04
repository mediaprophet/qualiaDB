pub use etalon_agent::device_caps::{GpuAdapterCaps, GpuFeatureCaps, GpuLimitCaps};
/// Feature set to request for native LLM inference, intersected with `available` so nothing unsupported is requested.
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

/// `true` when experimental GPU features may be requested (build/env policy gate).
pub fn experimental_features_allowed() -> bool {
    matches!(
        std::env::var("QUALIA_WGPU_EXPERIMENTAL_FEATURES")
            .ok()
            .as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

// ── Inference-pipeline (GPU backend) selection — the "which pipeline for this machine" checker ──

/// An explicit GPU-backend override for the inference device.
///
/// Priority:
/// 1. `QUALIA_WGPU_BACKEND` env (`vulkan` | `dx12` | `metal` | `gl` | `primary` | `all`)
/// 2. Cached [`crate::hardware_passport`] best GPU circuit (from `qualia-cli llm passport`)
/// 3. `None` → platform default in `init_shared_gpu_async` (Windows → DX12; else wgpu pick)
///
/// This is what lets a machine be pinned by measurement rather than a static hierarchy.
#[cfg(not(target_arch = "wasm32"))]
pub fn qualia_backend_override() -> Option<wgpu::Backends> {
    if let Ok(raw) = std::env::var("QUALIA_WGPU_BACKEND") {
        let v = raw.trim().to_ascii_lowercase();
        return match v.as_str() {
            "vulkan" | "vk" => Some(wgpu::Backends::VULKAN),
            "dx12" | "d3d12" | "directx12" => Some(wgpu::Backends::DX12),
            "metal" | "mtl" => Some(wgpu::Backends::METAL),
            "gl" | "opengl" | "gles" => Some(wgpu::Backends::GL),
            "primary" => Some(wgpu::Backends::PRIMARY),
            "all" => Some(wgpu::Backends::all()),
            other => {
                log::warn!(
                    "QUALIA_WGPU_BACKEND='{other}' unrecognized — trying passport / default"
                );
                None
            }
        }
        .or_else(passport_backend_override);
    }
    passport_backend_override()
}

/// Prefer the measured HardwarePassport GPU backend when no env pin is set.
#[cfg(not(target_arch = "wasm32"))]
fn passport_backend_override() -> Option<wgpu::Backends> {
    let backend = crate::hardware_passport::cached_preferred_wgpu_backend()?;
    let token = crate::hardware_passport::backend_env_token(&backend)?;
    log::info!("shared_gpu|backend_from_passport|{backend}|token={token}");
    match token {
        "vulkan" => Some(wgpu::Backends::VULKAN),
        "dx12" => Some(wgpu::Backends::DX12),
        "metal" => Some(wgpu::Backends::METAL),
        "gl" => Some(wgpu::Backends::GL),
        _ => None,
    }
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
        wgpu::Backend::BrowserWebGpu => "browser-webgpu — WebGPU in the page",
        _ => "noop/unknown — no GPU compute path",
    }
}

#[cfg(target_arch = "wasm32")]
/// Picks a recommended inference backend label from the adapter's real capabilities.
pub fn recommend_inference_backend(caps: &GpuAdapterCaps) -> &'static str {
    match caps.backend {
        wgpu::Backend::BrowserWebGpu => "browser-webgpu — WebGPU in the page (recommended in WASM)",
        _ => "browser-webgpu — request a WebGPU adapter (recommended in WASM)",
    }
}
