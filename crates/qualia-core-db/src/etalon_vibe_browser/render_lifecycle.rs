//! Device-harness-only browser execution of canonical render completion owners.
//! This deliberately does not qualify Portal's DOM/canvas replacement integration.
use etalon as gpu;
use etalon_render::{
    compute_readback::ComputeReadback, picking::PickResult, picking_readback::PickingReadback,
};
use etalon_wasm::readback::{ready_worker, SharedWorker};
use serde_json::{json, Value};
use wasm_bindgen::{prelude::*, JsCast};

async fn tick() -> Result<(), String> {
    let promise =
        js_sys::Function::new_no_args("return new Promise(resolve => setTimeout(resolve, 0))")
            .call0(&JsValue::NULL)
            .map_err(|e| format!("timer: {e:?}"))?;
    wasm_bindgen_futures::JsFuture::from(promise.unchecked_into::<js_sys::Promise>())
        .await
        .map_err(|e| format!("timer: {e:?}"))?;
    Ok(())
}

struct Work {
    pick: PickingReadback<()>,
    compute: ComputeReadback,
    // Keep submitted resources alive until completion or explicit teardown.
    _textures: [gpu::Texture; 2],
    _buffers: [gpu::Buffer; 2],
}

fn submit(device: &gpu::Device, queue: &gpu::Queue, worker: SharedWorker) -> Result<Work, String> {
    let texture = |format, usage| {
        device.create_texture(&gpu::TextureDescriptor {
            label: Some("render-lifecycle-1px"),
            size: gpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: gpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let color = texture(
        gpu::TextureFormat::R32Uint,
        gpu::TextureUsages::RENDER_ATTACHMENT | gpu::TextureUsages::COPY_SRC,
    );
    let depth = texture(
        gpu::TextureFormat::Depth32Float,
        gpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let staging = |size| {
        device.create_buffer(&gpu::BufferDescriptor {
            label: Some("render-lifecycle-map"),
            size,
            usage: gpu::BufferUsages::COPY_DST | gpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        })
    };
    let mut pick = PickingReadback::new(staging(256), (), 1);
    pick.queue(0.0, 0.0, 1, 1);
    let output = device.create_buffer(&gpu::BufferDescriptor {
        label: Some("render-lifecycle-compute"),
        size: 4,
        usage: gpu::BufferUsages::STORAGE | gpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let mapped = staging(4);
    let module = device.create_shader_module(gpu::ShaderModuleDescriptor {
        label: None, source: gpu::ShaderSource::Wgsl(
            "@group(0) @binding(0) var<storage, read_write> out: array<u32>; @compute @workgroup_size(1) fn main() { out[0] = 7u; }".into()),
    });
    let compute = device.create_compute_pipeline(&gpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let group = device.create_bind_group(&gpu::BindGroupDescriptor {
        label: None,
        layout: &compute.get_bind_group_layout(0),
        entries: &[gpu::BindGroupEntry {
            binding: 0,
            resource: output.as_entire_binding(),
        }],
    });
    // Real fragment output supplies tensor slot 0 (biased ID 1) to canonical picking.
    let shader = device.create_shader_module(gpu::ShaderModuleDescriptor { label: None,
        source: gpu::ShaderSource::Wgsl("@vertex fn vs(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32>{ var p=array<vec2<f32>,3>(vec2(-1.,-1.),vec2(3.,-1.),vec2(-1.,3.)); return vec4(p[i],0.,1.); } @fragment fn fs()->@location(0) u32 { return 1u; }".into()) });
    let render = device.create_render_pipeline(&gpu::RenderPipelineDescriptor {
        label: None,
        layout: None,
        vertex: gpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(gpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(gpu::ColorTargetState {
                format: gpu::TextureFormat::R32Uint,
                blend: None,
                write_mask: gpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(),
        depth_stencil: Some(gpu::DepthStencilState {
            format: gpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(gpu::CompareFunction::Always),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    etalon_render::picking_readback::record_pass(
        &mut encoder,
        &color.create_view(&Default::default()),
        &depth.create_view(&Default::default()),
        |pass| {
            pass.set_pipeline(&render);
            pass.draw(0..3, 0..1);
        },
    );
    pick.record_copy(&mut encoder, &color);
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&compute);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &mapped, 0, 4);
    queue.submit([encoder.finish()]);
    let compute = ComputeReadback::begin(&mapped, 4, worker)?;
    if compute.bytes() != 4 || pick.poll(device).is_some() {
        return Err("Expected real map callbacks still pending before yielding".into());
    }
    Ok(Work {
        pick,
        compute,
        _textures: [color, depth],
        _buffers: [output, mapped],
    })
}

async fn completed(work: &mut Work, device: &gpu::Device) -> Result<Value, String> {
    let mut pick = None;
    for _ in 0..200 {
        tick().await?;
        pick = pick.or_else(|| work.pick.poll(device));
        if work.compute.poll(device)? == Some(true) && pick.is_some() {
            let mut bytes = [0; 4];
            work.compute.copy_into(&mut bytes)?;
            if pick != Some(PickResult::Tensor(0)) || u32::from_le_bytes(bytes) != 7 {
                return Err(format!("Wrong actual GPU output: {pick:?}, {bytes:?}"));
            }
            return Ok(json!({"pick": 0, "compute": 7}));
        }
    }
    Err("GPU completion exceeded 200 browser timer turns".into())
}

async fn run(shared: bool, action: &str) -> Result<Value, String> {
    if !matches!(action, "cancel" | "teardown" | "device-loss") {
        return Err("Unknown action".into());
    }
    let instance = gpu::Instance::new(gpu::InstanceDescriptor {
        backends: gpu::Backends::BROWSER_WEBGPU,
        ..gpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = match instance
        .request_webgpu_adapter(&Default::default(), gpu::FeatureLevel::Compatibility)
        .await
    {
        Ok(adapter) => adapter,
        Err(error) => {
            return Ok(
                json!({"outcome":"unsupported", "reason":error.to_string(), "action":action}),
            )
        }
    };
    let info = adapter.get_info();
    if info.backend != gpu::Backend::BrowserWebGpu {
        return Err("Unexpected backend".into());
    }
    if info.device_type == gpu::DeviceType::Cpu {
        return Ok(json!({"outcome":"unsupported", "action":action,
            "reason":"Browser returned a CPU/software adapter; GPU lifecycle qualification refused",
            "adapter":{"name":info.name,"backend":format!("{:?}",info.backend),"deviceType":format!("{:?}",info.device_type)}}));
    }
    let (device, queue) = adapter
        .request_device(&Default::default())
        .await
        .map_err(|e| e.to_string())?;
    let scope = device.push_error_scope(gpu::ErrorFilter::Validation);
    let worker = ready_worker(shared);
    let canvas_exclusive = {
        let w = worker.lock().unwrap();
        w.attach_offscreen_canvas() && !w.attach_offscreen_canvas()
    };
    let old_stamp = worker.lock().unwrap().completion_stamp();
    let mut old = submit(&device, &queue, worker.clone())?;
    let token = worker
        .lock()
        .unwrap()
        .request_token(1)
        .ok_or("Missing pending compute request")?;
    // Invalidate before the browser gets a turn to deliver actual map callbacks.
    match action {
        "cancel" => {
            let mut w = worker.lock().unwrap();
            if !w.cancel_request(token) {
                return Err("Cancellation did not own request".into());
            }
            w.set_completion_context(1, 0)
                .map_err(|e| format!("{e:?}"))?;
            old.pick.invalidate(false);
        }
        "teardown" => {
            worker.lock().unwrap().terminate();
            old.pick.invalidate(false);
        }
        _ => {
            let lost_worker = worker.clone();
            let (sender, receiver) = futures_channel::oneshot::channel();
            let sender = std::sync::Mutex::new(Some(sender));
            device.set_device_lost_callback(move |_, _| {
                lost_worker.lock().unwrap().notify_device_lost();
                if let Some(sender) = sender.lock().unwrap().take() {
                    let _ = sender.send(());
                }
            });
            device.destroy();
            receiver.await.map_err(|_| "Device-loss callback dropped")?;
            old.pick.notify_device_lost();
        }
    }
    for _ in 0..200 {
        if old.compute.poll(&device)?.is_some() {
            break;
        }
        tick().await?;
    }
    let mut sentinel = [0xA5; 4];
    let rejected = old.compute.poll(&device)? == Some(false)
        && old.compute.copy_into(&mut sentinel).is_err()
        && sentinel == [0xA5; 4]
        && old.pick.poll(&device).is_none()
        && !worker.lock().unwrap().complete_request(token);
    drop(old);
    let replacement = if action == "teardown" {
        if worker.lock().unwrap().recover(1).is_ok() {
            return Err("Terminated owner recovered".into());
        }
        ready_worker(shared)
    } else {
        if action == "device-loss" {
            worker
                .lock()
                .unwrap()
                .recover(old_stamp.session + 1)
                .map_err(|e| format!("{e:?}"))?;
        }
        worker.lock().unwrap().detach_offscreen_canvas();
        worker.clone()
    };
    let canvas_reacquired = {
        let w = replacement.lock().unwrap();
        w.attach_offscreen_canvas() && !w.attach_offscreen_canvas()
    };
    // WebGPU adapter admission is single-use. Recovery needs a fresh adapter
    // request as well as a fresh device; retain the original invalidated owners.
    let fresh_adapter = match instance
        .request_webgpu_adapter(&Default::default(), gpu::FeatureLevel::Compatibility)
        .await
    {
        Ok(adapter) => adapter,
        Err(error) => {
            return Ok(json!({"outcome":"unsupported", "action":action,
            "phase":"recovery-adapter", "reason":error.to_string()}))
        }
    };
    let fresh_info = fresh_adapter.get_info();
    if fresh_info.backend != gpu::Backend::BrowserWebGpu {
        return Err("Unexpected recovery backend".into());
    }
    if fresh_info.device_type == gpu::DeviceType::Cpu {
        return Ok(json!({"outcome":"unsupported", "action":action,
            "phase":"recovery-adapter",
            "reason":"Recovery returned a CPU/software adapter; GPU lifecycle qualification refused",
            "adapter":{"name":fresh_info.name,"backend":format!("{:?}",fresh_info.backend),"deviceType":format!("{:?}",fresh_info.device_type)}}));
    }
    let (fresh, queue2) = fresh_adapter
        .request_device(&Default::default())
        .await
        .map_err(|e| e.to_string())?;
    let fresh_scope = fresh.push_error_scope(gpu::ErrorFilter::Validation);
    let mut work = submit(&fresh, &queue2, replacement.clone())?;
    let output = completed(&mut work, &fresh).await?;
    drop(work);
    replacement.lock().unwrap().terminate();
    let validation = fresh_scope.pop().await;
    fresh.destroy();
    let old_validation = scope.pop().await;
    device.destroy();
    if !rejected
        || !canvas_exclusive
        || !canvas_reacquired
        || validation.is_some()
        || old_validation.is_some()
    {
        return Err(format!("Lifecycle assertion failed: rejected={rejected}, canvas={canvas_exclusive}/{canvas_reacquired}, validation={validation:?}/{old_validation:?}"));
    }
    Ok(
        json!({"outcome":"passed", "action":action, "transportShared":shared,
        "adapter": {"name":info.name,"backend":format!("{:?}",info.backend),"deviceType":format!("{:?}",info.device_type),"vendor":info.vendor,"device":info.device},
        "recoveryAdapter": {"name":fresh_info.name,"backend":format!("{:?}",fresh_info.backend),"deviceType":format!("{:?}",fresh_info.device_type),"vendor":fresh_info.vendor,"device":fresh_info.device},
        "oldMapRejected":rejected,"pendingBeforeYield":true,"canvasExclusive":canvas_exclusive,
        "canvasReacquired":canvas_reacquired,"output":output,
        "canvasScope":"canonical stewardship admission only; GPU render uses an independent offscreen texture, not the transferred OffscreenCanvas",
        "hardwareClassification":"browser-reported adapter; physical GPU identity not independently verified",
        "scope":"canonical render/picking/compute owners; Portal DOM replacement not exercised"}),
    )
}

#[wasm_bindgen(js_name = runRenderLifecycleHarness)]
pub async fn harness(shared: bool, action: String) -> String {
    match run(shared, &action).await {
        Ok(value) => value.to_string(),
        Err(error) => json!({"outcome":"failed","action":action,"error":error}).to_string(),
    }
}
