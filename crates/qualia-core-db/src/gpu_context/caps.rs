pub use etalon_agent::device_caps::{GpuAdapterCaps, GpuFeatureCaps, GpuLimitCaps};
/// Feature set to request for native LLM inference, intersected with `available` so nothing unsupported is requested.
pub fn requested_native_llm_features(available: etalon::Features) -> etalon::Features {
    let mut desired = etalon::Features::TIMESTAMP_QUERY
        | etalon::Features::TIMESTAMP_QUERY_INSIDE_PASSES
        | etalon::Features::PIPELINE_STATISTICS_QUERY
        | etalon::Features::PIPELINE_CACHE
        | etalon::Features::SHADER_F16
        | etalon::Features::SUBGROUP
        | etalon::Features::SUBGROUP_BARRIER;
    if experimental_features_allowed() {
        desired |= etalon::Features::EXPERIMENTAL_COOPERATIVE_MATRIX;
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
pub fn qualia_backend_override() -> Option<etalon::Backends> {
    if let Ok(raw) = std::env::var("QUALIA_WGPU_BACKEND") {
        let v = raw.trim().to_ascii_lowercase();
        return match v.as_str() {
            "vulkan" | "vk" => Some(etalon::Backends::VULKAN),
            "dx12" | "d3d12" | "directx12" => Some(etalon::Backends::DX12),
            "metal" | "mtl" => Some(etalon::Backends::METAL),
            "gl" | "opengl" | "gles" => Some(etalon::Backends::GL),
            "primary" => Some(etalon::Backends::PRIMARY),
            "all" => Some(etalon::Backends::all()),
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
fn passport_backend_override() -> Option<etalon::Backends> {
    let backend = crate::hardware_passport::cached_preferred_wgpu_backend()?;
    let token = crate::hardware_passport::backend_env_token(&backend)?;
    log::info!("shared_gpu|backend_from_passport|{backend}|token={token}");
    match token {
        "vulkan" => Some(etalon::Backends::VULKAN),
        "dx12" => Some(etalon::Backends::DX12),
        "metal" => Some(etalon::Backends::METAL),
        "gl" => Some(etalon::Backends::GL),
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
        etalon::Backend::Vulkan => "vulkan — vendor-neutral & portable (recommended)",
        etalon::Backend::Metal => "metal — Apple-native (recommended on macOS)",
        etalon::Backend::Dx12 => {
            "dx12 — Windows-native; set QUALIA_WGPU_BACKEND=vulkan for the portable path"
        }
        etalon::Backend::Gl => "gl — compatibility fallback, limited compute (last resort)",
        etalon::Backend::BrowserWebGpu => "browser-webgpu — WebGPU in the page",
        _ => "noop/unknown — no GPU compute path",
    }
}

#[cfg(target_arch = "wasm32")]
/// Picks a recommended inference backend label from the adapter's real capabilities.
pub fn recommend_inference_backend(caps: &GpuAdapterCaps) -> &'static str {
    match caps.backend {
        etalon::Backend::BrowserWebGpu => "browser-webgpu — WebGPU in the page (recommended in WASM)",
        _ => "browser-webgpu — request a WebGPU adapter (recommended in WASM)",
    }
}
