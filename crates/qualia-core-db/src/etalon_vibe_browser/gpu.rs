use super::{case, oracle};
use crate::vibe_host::VibeHost;
use etalon::util::DeviceExt;
use serde_json::{json, Value as Json};

pub(super) async fn run() -> Json {
    let navigator = js_sys::Reflect::get(&js_sys::global(), &"navigator".into()).ok();
    let gpu_api = navigator
        .and_then(|navigator| js_sys::Reflect::get(&navigator, &"gpu".into()).ok())
        .is_some_and(|gpu| !gpu.is_null() && !gpu.is_undefined());
    let instance = etalon::Instance::new(etalon::InstanceDescriptor {
        backends: etalon::Backends::BROWSER_WEBGPU,
        ..etalon::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = match instance
        .request_webgpu_adapter(&Default::default(), etalon::FeatureLevel::Compatibility)
        .await
    {
        Ok(adapter) => adapter,
        Err(error) => {
            return json!({"name": "etalon_browser_compute_readback", "execution_class": "gpu",
            "outcome": "unsupported", "details": {"reason": if gpu_api {
                format!("Browser returned no compatibility WebGPU adapter: {error}")
            } else {
                "This browser does not expose WebGPU in the diagnostics worker".to_string()
            }, "webgpu_api_available": gpu_api, "requested_feature_level": "compatibility",
                "execution_path": "Rust Etalon BrowserWebGpu adapter request", "gpu_execution": false}})
        }
    };
    let info = adapter.get_info();
    if info.backend != etalon::Backend::BrowserWebGpu {
        return case(
            "etalon_browser_compute_readback",
            "gpu",
            Err("unexpected backend".into()),
        );
    }
    let limits = etalon::Limits {
        max_bind_groups: 1,
        max_bindings_per_bind_group: 3,
        max_storage_buffers_per_shader_stage: 3,
        max_storage_buffer_binding_size: 1028,
        max_buffer_size: 1028,
        max_compute_invocations_per_workgroup: 64,
        max_compute_workgroup_size_x: 64,
        max_compute_workgroup_size_y: 1,
        max_compute_workgroup_size_z: 1,
        max_compute_workgroups_per_dimension: 5,
        ..etalon::Limits::downlevel_webgl2_defaults()
    };
    let (device, queue) = match adapter
        .request_device(&etalon::DeviceDescriptor {
            required_limits: limits,
            ..Default::default()
        })
        .await
    {
        Ok(pair) => pair,
        Err(error) => {
            return case(
                "etalon_browser_compute_readback",
                "gpu",
                Err(error.to_string()),
            )
        }
    };
    let result = verify(&device, &queue).await.map(|dispatches| json!({
        "actualbackend": format!("{:?}", info.backend), "adapter_name": info.name,
        "requested_feature_level": "compatibility", "webgpu_api_available": gpu_api,
        "device_type": format!("{:?}", info.device_type), "vendor": info.vendor, "device": info.device,
        "hardware_classification": "browser-reported; physical GPU type is not independently verified",
        "execution_path": "canonical VibeHost CPU oracle -> Rust Etalon BrowserWebGpu -> browser adapter -> async map/readback",
        "element_counts": [1,63,64,65,257], "dispatches": dispatches,
        "sparse_zero_initialization_verified": true, "submission_index": null,
        "native_render_gpu_capabilities": "unsupported on this browser bridge",
        "native_hal_or_naga_coverage": false,
    }));
    device.destroy();
    case("etalon_browser_compute_readback", "gpu", result)
}

async fn verify(device: &etalon::Device, queue: &etalon::Queue) -> Result<usize, String> {
    let mut host = VibeHost::with_seed(vec![]);
    let mut dispatches = 0;
    for count in [1, 63, 64, 65, 257] {
        for offset in [0.0, 7.0] {
            dispatch(device, queue, &mut host, count, offset, false).await?;
            dispatches += 1;
        }
    }
    dispatch(device, queue, &mut host, 65, 0.0, true).await?;
    Ok(dispatches + 1)
}

async fn dispatch(
    device: &etalon::Device,
    queue: &etalon::Queue,
    host: &mut VibeHost,
    count: usize,
    offset: f32,
    sparse: bool,
) -> Result<(), String> {
    let a: Vec<f32> = (0..count).map(|i| i as f32 * 0.25 + offset).collect();
    let b: Vec<f32> = (0..count).map(|i| 16.0 - i as f32 * 0.5).collect();
    let expected: Vec<f32> = a
        .iter()
        .zip(&b)
        .enumerate()
        .map(|(i, (&a, &b))| {
            if sparse && i % 2 == 1 {
                Ok(0.0)
            } else {
                oracle(host, a, b)
            }
        })
        .collect::<Result<_, _>>()?;
    let scope = device.push_error_scope(etalon::ErrorFilter::Validation);
    let input = |values: &[f32]| {
        device.create_buffer_init(&etalon::util::BufferInitDescriptor {
            label: Some("browser-harness-input"),
            contents: bytemuck::cast_slice(values),
            usage: etalon::BufferUsages::STORAGE,
        })
    };
    let input_a = input(&a);
    let input_b = input(&b);
    let size = (count * 4) as u64;
    let output = device.create_buffer(&etalon::BufferDescriptor {
        label: Some("browser-harness-output"),
        size,
        usage: etalon::BufferUsages::STORAGE | etalon::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging = device.create_buffer(&etalon::BufferDescriptor {
        label: Some("browser-harness-readback"),
        size,
        usage: etalon::BufferUsages::MAP_READ | etalon::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let module = device.create_shader_module(etalon::ShaderModuleDescriptor {
        label: Some("browser-harness-vector-add"),
        source: etalon::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
    });
    let pipeline = device.create_compute_pipeline(&etalon::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &module,
        entry_point: Some(if sparse { "sparse" } else { "add" }),
        compilation_options: Default::default(),
        cache: None,
    });
    let group = device.create_bind_group(&etalon::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            etalon::BindGroupEntry {
                binding: 0,
                resource: input_a.as_entire_binding(),
            },
            etalon::BindGroupEntry {
                binding: 1,
                resource: input_b.as_entire_binding(),
            },
            etalon::BindGroupEntry {
                binding: 2,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups((count as u32).div_ceil(64), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &staging, 0, size);
    if queue.submit([encoder.finish()]).is_some() {
        return Err("browser unexpectedly returned a native submission index".into());
    }
    if let Some(error) = scope.pop().await {
        return Err(format!("Etalon validation failed: {error}"));
    }
    let (sender, receiver) = futures_channel::oneshot::channel();
    staging
        .slice(..)
        .map_async(etalon::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    receiver
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    let mapped = staging
        .slice(..)
        .get_mapped_range()
        .map_err(|error| error.to_string())?;
    let actual: Vec<f32> = mapped
        .chunks_exact(4)
        .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()))
        .collect();
    drop(mapped);
    staging.unmap();
    if actual != expected {
        return Err(format!("readback differs from canonical Vibe oracle: count={count}, offset={offset}, sparse={sparse}"));
    }
    Ok(())
}
