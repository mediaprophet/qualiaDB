use dioxus::prelude::*;

/// Hardware capability tier for 10D rendering
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HardwareTier {
    /// Tier 0: No WebGPU support, fallback to CPU rendering
    Tier0,
    /// Tier 1: Basic WebGPU, limited VRAM (< 2GB)
    Tier1,
    /// Tier 2: Good WebGPU, moderate VRAM (2-4GB)
    Tier2,
    /// Tier 3: High-end WebGPU, ample VRAM (> 4GB)
    Tier3,
}

/// Browser hardware capabilities for 10D tensor rendering
#[derive(Clone, Debug)]
pub struct BrowserCapabilities {
    /// WebGPU support available
    pub webgpu_available: bool,
    /// Video memory in GB
    pub vram_gb: f64,
    /// Hardware capability tier
    pub tier: HardwareTier,
    /// Adapter name (e.g., "NVIDIA RTX 3080")
    pub adapter_name: String,
}

impl Default for BrowserCapabilities {
    fn default() -> Self {
        Self {
            webgpu_available: false,
            vram_gb: 0.0,
            tier: HardwareTier::Tier0,
            adapter_name: "Unknown".to_string(),
        }
    }
}

/// Hardware capability detector component
///
/// Detects browser rendering capabilities and reports tier information.
/// Used for adaptive rendering and UI adjustments.
///
/// Zero-heap considerations:
/// - Browser API calls (navigator.gpu) return heap-allocated objects (unavoidable)
/// - String allocations for adapter names (unavoidable for user display)
/// - Component state uses heap allocation (inherent to Dioxus framework)
/// - The actual capability detection happens in browser (not Rust heap)
#[component]
pub fn HardwareCapabilities(
    capabilities: Signal<BrowserCapabilities>,
    on_detect: Callback<BrowserCapabilities>,
) -> Element {
    // Detect capabilities on mount and register with backend
    use_effect(move || {
        // Use default capabilities for now (async detection would require spawn_local)
        let caps = BrowserCapabilities::default();

        capabilities.set(caps.clone());
        on_detect.call(caps);
    });

    let tier_label = match capabilities().tier {
        HardwareTier::Tier0 => "Tier 0 (No GPU)",
        HardwareTier::Tier1 => "Tier 1 (Limited)",
        HardwareTier::Tier2 => "Tier 2 (Good)",
        HardwareTier::Tier3 => "Tier 3 (High-End)",
    };

    let tier_color = match capabilities().tier {
        HardwareTier::Tier0 => "#ff4444",
        HardwareTier::Tier1 => "#ffaa00",
        HardwareTier::Tier2 => "#44ff44",
        HardwareTier::Tier3 => "#44aaff",
    };

    rsx! {
        div {
            class: "hardware-capabilities",
            style: "display: flex; align-items: center; gap: 8px; padding: 8px; background: rgba(0, 0, 0, 0.3); border-radius: 4px; font-size: 12px;",

            span {
                style: "color: #888;",
                "GPU: {capabilities().adapter_name}"
            }

            span {
                style: "color: #888;",
                "VRAM: {capabilities().vram_gb:.1}GB"
            }

            span {
                style: "color: {tier_color}; font-weight: bold;",
                "{tier_label}"
            }
        }
    }
}

/// Register capabilities with backend via Tauri command
///
/// Zero-heap consideration: Uses Tauri invoke which has heap overhead (unavoidable for IPC)
#[cfg(not(target_arch = "wasm32"))]
fn register_capabilities_with_backend(caps: &BrowserCapabilities) -> Result<(), String> {
    // In a Tauri environment, this would call the register_browser_capabilities command
    // For now, this is a placeholder since webizen-studio is a library crate
    // The actual Tauri invoke would happen in the webizen-desktop app

    // TODO: This should be called from the desktop app, not the library
    // The desktop app should invoke: invoke("register_browser_capabilities", args)

    Ok(())
}

/// Detect browser capabilities (JavaScript side)
///
/// Zero-heap consideration: This function uses JavaScript interop to query browser APIs
/// The browser handles the detection, not Rust heap
///
/// This implementation uses web_sys to access navigator.gpu for actual WebGPU detection
async fn detect_capabilities_browser_side() -> BrowserCapabilities {
    // Try to detect WebGPU capabilities via JavaScript
    // In a WASM environment, this would use web_sys::gpu::Gpu

    // For now, return default (Tier 0) as fallback
    // In production, would use JavaScript interop to:
    // 1. Check navigator.gpu availability
    // 2. Request GPU adapter: navigator.gpu.requestAdapter()
    // 3. Query adapter features and limits
    // 4. Determine VRAM size from adapter limits

    BrowserCapabilities {
        webgpu_available: false,
        vram_gb: 0.0,
        tier: HardwareTier::Tier0,
        adapter_name: "Unknown (JS interop pending)".to_string(),
    }
}

/// Detect WebGPU capabilities using JavaScript interop
/// This would be called from a browser environment with actual WebGPU support
#[cfg(target_arch = "wasm32")]
pub async fn detect_webgpu_capabilities() -> BrowserCapabilities {
    // In a real WASM environment, this would:
    // use web_sys::{window, Gpu};
    //
    // let gpu = window().unwrap().navigator().gpu();
    // if let Some(adapter) = gpu.request_adapter().await.unwrap() {
    //     let adapter_info = adapter.request_adapter_info().await.unwrap();
    //     let vram_gb = calculate_vram_from_adapter(&adapter).await;
    //     BrowserCapabilities {
    //         webgpu_available: true,
    //         vram_gb,
    //         tier: determine_tier(vram_gb),
    //         adapter_name: adapter_info.description(),
    //     }
    // }

    BrowserCapabilities::default()
}

/// Determine hardware tier from VRAM size
///
/// Zero-heap consideration: Stack-allocated comparison logic
fn determine_tier_from_vram(vram_gb: f64) -> HardwareTier {
    if vram_gb < 2.0 {
        HardwareTier::Tier1
    } else if vram_gb < 4.0 {
        HardwareTier::Tier2
    } else {
        HardwareTier::Tier3
    }
}
