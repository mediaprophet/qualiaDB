//! Native renderer-only adapter selection.
//!
//! The renderer normally borrows the process device. If that compute-oriented
//! device is unavailable, this path selects a graphics adapter without changing
//! inference's `QUALIA_WGPU_BACKEND` choice.

use std::sync::Arc;

use etalon::{Backends, Device, InstanceDescriptor, PowerPreference, Queue};

/// Explicit renderer pin. `None` means use the graphics fallback order.
pub(super) fn backend_override() -> Result<Option<Backends>, String> {
    let Ok(raw) = std::env::var("QUALIA_RENDER_WGPU_BACKEND") else {
        return Ok(None);
    };
    parse_backend_override(&raw)
}

fn parse_backend_override(raw: &str) -> Result<Option<Backends>, String> {
    let backend = match raw.trim().to_ascii_lowercase().as_str() {
        "auto" => return Ok(None),
        "dx12" | "d3d12" => Backends::DX12,
        "vulkan" | "vk" => Backends::VULKAN,
        "gl" | "gles" | "opengl" => Backends::GL,
        "metal" => Backends::METAL,
        other => {
            return Err(format!(
                "unknown QUALIA_RENDER_WGPU_BACKEND '{other}'; expected auto, dx12, vulkan, gl, or metal"
            ));
        }
    };
    Ok(Some(backend))
}

fn fallback_backends(is_windows: bool, is_macos: bool) -> ([Backends; 3], usize) {
    if is_windows {
        #[cfg(feature = "gpu-native-dx12")]
        {
            // Prefer DX12, then Vulkan (which respects DiscreteGpu selection), then GL last.
            ([Backends::DX12, Backends::VULKAN, Backends::GL], 3)
        }
        #[cfg(not(feature = "gpu-native-dx12"))]
        {
            // Prefer Vulkan before GL on Windows portable builds so DiscreteGpu
            // (NVIDIA/AMD) is selected instead of GL's default Intel integrated GPU context.
            ([Backends::VULKAN, Backends::GL, Backends::METAL], 3)
        }
    } else if is_macos {
        ([Backends::METAL, Backends::GL, Backends::VULKAN], 3)
    } else {
        ([Backends::VULKAN, Backends::GL, Backends::METAL], 3)
    }
}

/// GPU adapter preference (`QUALIA_GPU_PREFERENCE` / `SYNTH_GPU_PREFERENCE`):
/// - `"discrete"` / `"high-performance"` (default): strictly prefer `DeviceType::DiscreteGpu`
///   and reject `DeviceType::IntegratedGpu` when `QUALIA_REJECT_IGPU=1` or when a discrete
///   GPU exists on another backend.
/// - `"integrated"` / `"low-power"`: select `DeviceType::IntegratedGpu`.
/// - `"name:<substring>"` / `"index:<n>"`: match adapter name or index.
fn select_preferred_adapter(
    instance: &etalon::Instance,
    backend: Backends,
) -> Result<etalon::Adapter, String> {
    let pref = std::env::var("SYNTH_GPU_PREFERENCE")
        .or_else(|_| std::env::var("QUALIA_GPU_PREFERENCE"))
        .unwrap_or_else(|_| "discrete".to_string())
        .trim()
        .to_ascii_lowercase();
    let reject_igpu = std::env::var("SYNTH_REJECT_IGPU")
        .or_else(|_| std::env::var("QUALIA_REJECT_IGPU"))
        .map(|v| matches!(v.trim(), "1" | "true" | "yes"))
        .unwrap_or(pref == "discrete-only" || pref == "no-igpu");

    let adapters = pollster::block_on(instance.enumerate_adapters(backend));
    if !adapters.is_empty() {
        if let Some(sub) = pref.strip_prefix("name:") {
            let needle = sub.trim();
            if let Some(found) = adapters
                .iter()
                .find(|a| a.get_info().name.to_ascii_lowercase().contains(needle))
            {
                return Ok(found.clone());
            }
        }
        if pref == "integrated" || pref == "igpu" || pref == "low-power" {
            if let Some(found) = adapters
                .iter()
                .find(|a| a.get_info().device_type == etalon::DeviceType::IntegratedGpu)
            {
                return Ok(found.clone());
            }
        } else {
            // Default: always pick DiscreteGpu first so we never accidentally throttle the Intel iGPU!
            if let Some(found) = adapters
                .iter()
                .find(|a| a.get_info().device_type == etalon::DeviceType::DiscreteGpu)
            {
                return Ok(found.clone());
            }
            if reject_igpu {
                return Err(format!(
                    "no DiscreteGpu found on {backend:?} and integrated GPU is disabled (SYNTH_GPU_PREFERENCE={pref})"
                ));
            }
        }
    }

    let power_preference = if pref == "integrated" || pref == "igpu" || pref == "low-power" {
        PowerPreference::LowPower
    } else {
        PowerPreference::HighPerformance
    };
    let adapter = match pollster::block_on(instance.request_adapter(
        &etalon::RequestAdapterOptions {
            power_preference,
            ..Default::default()
        },
    )) {
        Ok(a) => a,
        Err(primary_err) => pollster::block_on(instance.request_adapter(
            &etalon::RequestAdapterOptions {
                power_preference: PowerPreference::None,
                ..Default::default()
            },
        ))
        .map_err(|fallback_err| {
            format!("{backend:?}: primary: {primary_err}; fallback: {fallback_err}")
        })?,
    };
    if reject_igpu && adapter.get_info().device_type == etalon::DeviceType::IntegratedGpu {
        return Err(format!(
            "rejected IntegratedGpu '{}' on {backend:?} (discrete-only policy active)",
            adapter.get_info().name
        ));
    }
    Ok(adapter)
}

/// Create an offscreen renderer device, trying bounded native backends in order.
///
/// Adapter and device creation is cold initialization. A caller-specified backend
/// is strict: failure is reported instead of silently selecting another device.
pub(super) fn request_offscreen_device() -> Result<(Arc<Device>, Arc<Queue>), String> {
    let explicit = backend_override()?;
    let (fallbacks, fallback_count) =
        fallback_backends(cfg!(target_os = "windows"), cfg!(target_os = "macos"));
    let (candidates, count) = if let Some(backend) = explicit {
        ([backend, Backends::empty(), Backends::empty()], 1)
    } else {
        (fallbacks, fallback_count)
    };

    let mut failures = Vec::with_capacity(count);
    for backend in candidates.into_iter().take(count) {
        let mut descriptor = InstanceDescriptor::new_without_display_handle();
        descriptor.backends = backend;
        let instance = etalon::Instance::new(descriptor);
        let adapter = match select_preferred_adapter(&instance, backend) {
            Ok(adapter) => adapter,
            Err(err) => {
                failures.push(err);
                continue;
            }
        };

        let info = adapter.get_info();
        match pollster::block_on(adapter.request_device(&etalon::DeviceDescriptor {
            label: Some("Qualia native render device"),
            required_features: adapter.features() & etalon::Features::TIMESTAMP_QUERY,
            required_limits: adapter.limits(),
            ..Default::default()
        })) {
            Ok((device, queue)) => {
                log::info!(
                    "renderer_gpu|selected|backend={:?}|adapter={}|device={:?}",
                    info.backend,
                    info.name,
                    info.device_type
                );
                return Ok((Arc::new(device), Arc::new(queue)));
            }
            Err(error) => failures.push(format!(
                "{backend:?} ({}): device request: {error}",
                info.name
            )),
        }
    }

    Err(format!(
        "no native renderer adapter could be initialized: {}",
        failures.join("; ")
    ))
}

#[cfg(test)]
mod tests {
    use super::{fallback_backends, parse_backend_override, Backends};

    #[test]
    fn render_backend_pin_is_strict_and_independent() {
        assert_eq!(parse_backend_override(" GL ").unwrap(), Some(Backends::GL));
        assert_eq!(
            parse_backend_override("vk").unwrap(),
            Some(Backends::VULKAN)
        );
        assert_eq!(parse_backend_override("auto").unwrap(), None);
        assert!(parse_backend_override("automatic").is_err());
    }

    #[test]
    fn windows_renderer_fallback_uses_configured_backend_order() {
        let (backends, count) = fallback_backends(true, false);
        assert_eq!(count, 3);
        #[cfg(feature = "gpu-native-dx12")]
        assert_eq!(backends, [Backends::DX12, Backends::VULKAN, Backends::GL]);
        #[cfg(not(feature = "gpu-native-dx12"))]
        assert_eq!(backends, [Backends::VULKAN, Backends::GL, Backends::METAL]);
    }

    #[test]
    fn portable_fallback_orders_match_platform_native_apis() {
        assert_eq!(
            fallback_backends(false, true).0,
            [Backends::METAL, Backends::GL, Backends::VULKAN]
        );
        assert_eq!(
            fallback_backends(false, false).0,
            [Backends::VULKAN, Backends::GL, Backends::METAL]
        );
    }
}
