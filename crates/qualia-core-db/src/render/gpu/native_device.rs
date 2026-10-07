//! Native renderer-only adapter selection.
//!
//! The renderer normally borrows the process device. If that compute-oriented
//! device is unavailable, this path selects a graphics adapter without changing
//! inference's `QUALIA_WGPU_BACKEND` choice.

use std::sync::Arc;

use wgpu::{Backends, Device, InstanceDescriptor, PowerPreference, Queue};

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
            // Prefer the compute/render device where available. GL avoids routing
            // graphics fallback through the known-problematic Vulkan inference path.
            ([Backends::DX12, Backends::GL, Backends::VULKAN], 3)
        }
        #[cfg(not(feature = "gpu-native-dx12"))]
        {
            ([Backends::GL, Backends::VULKAN, Backends::METAL], 3)
        }
    } else if is_macos {
        ([Backends::METAL, Backends::GL, Backends::VULKAN], 3)
    } else {
        ([Backends::VULKAN, Backends::GL, Backends::METAL], 3)
    }
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
        let instance = wgpu::Instance::new(descriptor);
        let adapter = match pollster::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                ..Default::default()
            },
        )) {
            Ok(adapter) => adapter,
            Err(high_performance_error) => {
                match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: PowerPreference::LowPower,
                    ..Default::default()
                })) {
                    Ok(adapter) => adapter,
                    Err(low_power_error) => {
                        failures.push(format!(
                        "{backend:?}: high-performance: {high_performance_error}; low-power: {low_power_error}"
                    ));
                        continue;
                    }
                }
            }
        };

        let info = adapter.get_info();
        match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Qualia native render device"),
            required_features: wgpu::Features::empty(),
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
        assert_eq!(backends, [Backends::DX12, Backends::GL, Backends::VULKAN]);
        #[cfg(not(feature = "gpu-native-dx12"))]
        assert_eq!(backends, [Backends::GL, Backends::VULKAN, Backends::METAL]);
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
