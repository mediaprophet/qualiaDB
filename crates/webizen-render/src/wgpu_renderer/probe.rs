//! Bounded WebGPU adapter/device capability probing.

use super::{
    bounded_capability_text, PixelFormat, WebGpuAdapterReport, WebGpuCapabilityReport,
    WebGpuFeatureReport, WebGpuLimitReport, WebGpuStatus, MAX_OFFSCREEN_DIMENSION,
    MAX_PIXEL_READBACK_BYTES,
};

pub(crate) fn capability_report_from_adapter(
    adapter: &etalon::Adapter,
    device: Option<&etalon::Device>,
) -> WebGpuCapabilityReport {
    let info = adapter.get_info();
    let features = device
        .map(etalon::Device::features)
        .unwrap_or_else(|| adapter.features());
    let limits = device
        .map(etalon::Device::limits)
        .unwrap_or_else(|| adapter.limits());
    let status = if device.is_some() {
        WebGpuStatus::Ready
    } else {
        WebGpuStatus::AdapterOnly
    };

    WebGpuCapabilityReport {
        status,
        adapter_available: true,
        device_available: device.is_some(),
        adapter: Some(WebGpuAdapterReport {
            name: bounded_capability_text(info.name),
            backend: backend_label(info.backend).to_string(),
            device_type: device_type_label(info.device_type).to_string(),
            vendor: info.vendor,
            device: info.device,
            driver: bounded_capability_text(info.driver),
            driver_info: bounded_capability_text(info.driver_info),
            features: WebGpuFeatureReport {
                shader_f16: features.contains(etalon::Features::SHADER_F16),
                subgroup: features.contains(etalon::Features::SUBGROUP),
                timestamp_query: features.contains(etalon::Features::TIMESTAMP_QUERY),
                texture_compression_bc: features
                    .contains(etalon::Features::TEXTURE_COMPRESSION_BC),
            },
            limits: WebGpuLimitReport {
                max_texture_dimension_2d: limits.max_texture_dimension_2d,
                max_buffer_size: limits.max_buffer_size,
            },
        }),
        offscreen_format: PixelFormat::Rgba8Unorm,
        max_offscreen_dimension: MAX_OFFSCREEN_DIMENSION,
        max_pixel_readback_bytes: MAX_PIXEL_READBACK_BYTES,
    }
}

fn backend_label(backend: etalon::Backend) -> &'static str {
    match backend {
        etalon::Backend::Noop => "noop",
        etalon::Backend::Vulkan => "vulkan",
        etalon::Backend::Metal => "metal",
        etalon::Backend::Dx12 => "dx12",
        etalon::Backend::Gl => "gl",
        etalon::Backend::BrowserWebGpu => "browser-webgpu",
    }
}

fn device_type_label(device_type: etalon::DeviceType) -> &'static str {
    match device_type {
        etalon::DeviceType::Other => "other",
        etalon::DeviceType::IntegratedGpu => "integrated",
        etalon::DeviceType::DiscreteGpu => "discrete",
        etalon::DeviceType::VirtualGpu => "virtual",
        etalon::DeviceType::Cpu => "cpu",
    }
}

/// Probe the adapter/device path without constructing a renderer target.
///
/// This is safe for callers that can fall back when WebGPU is absent. The
/// report is deliberately bounded and contains no wgpu handles.
pub async fn probe_webgpu_capabilities() -> WebGpuCapabilityReport {
    #[cfg(feature = "qualia")]
    if let Some(shared) = qualia_core_db::gpu_context::try_shared_gpu() {
        return capability_report_from_adapter(&shared.adapter, Some(&shared.device));
    }

    let instance = etalon::Instance::default();
    let adapter = match instance
        .request_adapter(&etalon::RequestAdapterOptions {
            power_preference: etalon::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        })
        .await
    {
        Ok(adapter) => adapter,
        Err(_) => return WebGpuCapabilityReport::unavailable(),
    };

    match adapter
        .request_device(&etalon::DeviceDescriptor {
            label: Some("webizen-render-capability-probe"),
            required_features: etalon::Features::empty(),
            required_limits: etalon::Limits::downlevel_defaults(),
            ..Default::default()
        })
        .await
    {
        Ok((device, _queue)) => capability_report_from_adapter(&adapter, Some(&device)),
        Err(_) => WebGpuCapabilityReport {
            status: WebGpuStatus::DeviceUnavailable,
            ..capability_report_from_adapter(&adapter, None)
        },
    }
}
