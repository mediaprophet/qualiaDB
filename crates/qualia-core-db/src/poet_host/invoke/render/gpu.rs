//! WebGPU `capability.invoke` surface — exposes `PortalGpu` to VibeScript.
//!
//! These handlers are Tier-2 (cold construction): they marshal Vibe `Value`
//! arguments into GPU types and dispatch to the existing `render::gpu::PortalGpu`.
//! The render frame loop itself (`gpu_render_frame`) calls `PortalGpu::render`
//! which is zero-heap in its hot path.
//!
//! ## Invoke IDs
//!
//! | ID | Description |
//! |---|---|
//! | `Render.gpu_adapter_info` | Query the shared GPU adapter |
//! | `Render.gpu_init` | Create a PortalGpu instance (offscreen native) |
//! | `Render.gpu_render_frame` | Render one frame |
//! | `Render.gpu_read_pixels` | Read back RGBA8 pixels |
//! | `Render.gpu_upload_mesh` | Upload triangle mesh data |
//! | `Render.gpu_upload_tensor` | Upload tensor node buffer |
//! | `Render.gpu_set_camera` | Set camera orbit angles |
//! | `Render.gpu_pick` | Queue a pick query at screen coords |
//! | `Render.gpu_poll_pick` | Poll for pick result |
//! | `Render.gpu_resize` | Resize the viewport |
//! | `Render.gpu_set_ambient` | Enable/disable ambient particle field |

use super::super::args;
use poet_vibe::{Diagnostic, Span, Value};

use std::collections::BTreeMap;
use std::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use crate::gpu_context;
#[cfg(not(target_arch = "wasm32"))]
use crate::render::gpu::PortalGpu;
#[cfg(not(target_arch = "wasm32"))]
use crate::render::telemetry::SystemTelemetry;
/// Slot-map handle for a PortalGpu instance.
pub type GpuHandle = u64;

#[cfg(not(target_arch = "wasm32"))]
struct GpuSlot {
    portal: PortalGpu,
}

#[cfg(not(target_arch = "wasm32"))]
static GPU_SLOTS: Mutex<Vec<Option<GpuSlot>>> = Mutex::new(Vec::new());

/// Clear poisoned mutex and recover.
#[cfg(not(target_arch = "wasm32"))]
fn lock_slots() -> std::sync::MutexGuard<'static, Vec<Option<GpuSlot>>> {
    match GPU_SLOTS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// Find a slot index, allocating a new one if needed.
#[cfg(not(target_arch = "wasm32"))]
fn slot_insert(portal: PortalGpu) -> GpuHandle {
    let mut slots = lock_slots();
    for (i, entry) in slots.iter_mut().enumerate() {
        if entry.is_none() {
            *entry = Some(GpuSlot { portal });
            return i as GpuHandle;
        }
    }
    let i = slots.len();
    slots.push(Some(GpuSlot { portal }));
    i as GpuHandle
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn slot_with<R>(handle: GpuHandle, f: impl FnOnce(&mut PortalGpu) -> R) -> Option<R> {
    let mut slots = lock_slots();
    let idx = handle as usize;
    if idx >= slots.len() {
        return None;
    }
    slots[idx].as_mut().map(|s| f(&mut s.portal))
}

#[cfg(not(target_arch = "wasm32"))]
fn slot_remove(handle: GpuHandle) -> bool {
    let mut slots = lock_slots();
    let idx = handle as usize;
    if idx >= slots.len() {
        return false;
    }
    slots[idx].take().is_some()
}

// ── Invoke handlers ──────────────────────────────────────────────────────

/// `Render.gpu_adapter_info` — query the shared GPU adapter.
pub fn gpu_adapter_info(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let ctx = gpu_context::try_shared_gpu();
        let Some(ctx) = ctx else {
            return Ok(args::record([
                ("available", Value::Bool(false)),
                ("backend", Value::String("none".into())),
                ("device_name", Value::String("no GPU adapter".into())),
            ]));
        };
        let caps = &ctx.adapter_caps;
        Ok(args::record([
            ("available", Value::Bool(true)),
            ("backend", Value::String(caps.backend_label().into())),
            ("device_type", Value::String(caps.device_type_label().into())),
            ("device_name", Value::String(caps.name.clone())),
            ("driver", Value::String(caps.driver.clone())),
            ("driver_info", Value::String(caps.driver_info.clone())),
            ("vendor", Value::U64(caps.vendor as u64)),
            ("device_id", Value::U64(caps.device as u64)),
            ("shader_f16", Value::Bool(caps.features.shader_f16)),
            ("subgroup", Value::Bool(caps.features.subgroup)),
            ("cooperative_matrix", Value::Bool(caps.features.cooperative_matrix)),
            ("ray_query", Value::Bool(caps.features.ray_query)),
            ("max_buffer_size", Value::U64(caps.limits.max_buffer_size)),
            (
                "max_storage_buffer_binding_size",
                Value::U64(caps.limits.max_storage_buffer_binding_size),
            ),
            (
                "max_compute_workgroup_size_x",
                Value::U64(caps.limits.max_compute_workgroup_size_x as u64),
            ),
        ]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        Ok(args::record([
            ("available", Value::Bool(false)),
            ("backend", Value::String("wasm".into())),
            ("device_name", Value::String("WebGPU not available in wasm-logic".into())),
        ]))
    }
}

/// `Render.gpu_init` — create a PortalGpu offscreen instance.
///
/// On native: always creates a WebGPU PortalGpu. The `backend` field in the
/// return is always `"webgpu"`.
///
/// On WASM: probes WebGPU availability via `Render.gpu_backend_info`. If
/// WebGPU is available, creates a PortalGpu (browser WebGPU). If not, returns
/// an error indicating WebGL2 fallback is required (the WebGL2 path needs a
/// canvas element, which is handled by the portal facade, not the offscreen
/// invoke model).
pub fn gpu_init(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let width = args::rec_u64(args, "width").unwrap_or(800) as u32;
        let height = args::rec_u64(args, "height").unwrap_or(600) as u32;
        let particle_cap = args::rec_u64(args, "particle_cap").unwrap_or(5000) as usize;

        // Backend detection: on native, WebGPU is the only option. If no
        // adapter is available, fail with a clear error.
        let ctx = gpu_context::try_shared_gpu();
        if ctx.is_none() {
            return Err(args::bad(
                span,
                "gpu_init: no WebGPU adapter available (WebGL2 fallback is browser-only)",
            ));
        }

        let portal = PortalGpu::new_offscreen(width, height, particle_cap).map_err(|e| {
            args::bad(span, format!("gpu_init failed: {e}"))
        })?;
        let handle = slot_insert(portal);

        Ok(args::record([
            ("handle", Value::U64(handle)),
            ("width", Value::U64(width as u64)),
            ("height", Value::U64(height as u64)),
            ("backend", Value::String("webgpu".into())),
        ]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (args, span);
        Err(args::bad(span, "gpu_init requires native build with gpu-runtime; on WASM, use the portal facade for WebGPU/WebGL2 canvas rendering"))
    }
}

/// `Render.gpu_render_frame` — render one frame.
pub fn gpu_render_frame(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = args::rec_u64(args, "handle").ok_or_else(|| {
            args::bad(span, "gpu_render_frame needs { handle: u64 }")
        })?;
        let time = args::rec_f64(args, "time").unwrap_or(0.0) as f32;
        let telemetry = SystemTelemetry::default();

        slot_with(handle, |portal| {
            portal.render(time, &telemetry)
        })
        .ok_or_else(|| args::bad(span, "gpu_render_frame: invalid handle"))?
        .map_err(|e| args::bad(span, format!("gpu_render_frame: {e}")))?;

        Ok(args::record([
            ("rendered", Value::Bool(true)),
        ]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (args, span);
        Err(args::bad(span, "gpu_render_frame requires native build"))
    }
}

/// `Render.gpu_read_pixels` — read RGBA8 pixels from the offscreen target.
pub fn gpu_read_pixels(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = args::rec_u64(args, "handle").ok_or_else(|| {
            args::bad(span, "gpu_read_pixels needs { handle: u64 }")
        })?;

        slot_with(handle, |portal| {
            let need = portal.required_rgba8_bytes();
            let mut buf = vec![0u8; need];
            match portal.read_rgba8_into(&mut buf) {
                Ok(n) => Some((buf, n)),
                Err(_) => None,
            }
        })
        .ok_or_else(|| args::bad(span, "gpu_read_pixels: invalid handle"))?
        .map(|(buf, n)| {
            let (w, h) = slot_with(handle, |p| p.surface_size()).unwrap_or((0, 0));
            args::record([
                ("rgba8", Value::List(buf.into_iter().take(n).map(|b| Value::U64(b as u64)).collect())),
                ("width", Value::U64(w as u64)),
                ("height", Value::U64(h as u64)),
                ("bytes", Value::U64(n as u64)),
            ])
        })
        .ok_or_else(|| args::bad(span, "gpu_read_pixels: readback failed"))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (args, span);
        Err(args::bad(span, "gpu_read_pixels requires native build"))
    }
}

/// `Render.gpu_upload_mesh` — upload triangle mesh data.
pub fn gpu_upload_mesh(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = args::rec_u64(args, "handle").ok_or_else(|| {
            args::bad(span, "gpu_upload_mesh needs { handle: u64 }")
        })?;

        let pos_f64s = args::rec_f64_list(args, "positions")
            .ok_or_else(|| args::bad(span, "gpu_upload_mesh needs { positions: [f32] }"))?;
        let idx_u64s: Vec<u64> = args::rec(args, "indices")
            .and_then(|v| args::list(v))
            .map(|xs| xs.iter().filter_map(args::as_u64).collect())
            .ok_or_else(|| args::bad(span, "gpu_upload_mesh needs { indices: [u32] }"))?;

        let positions: Vec<[f32; 3]> = pos_f64s
            .chunks_exact(3)
            .map(|c| [c[0] as f32, c[1] as f32, c[2] as f32])
            .collect();
        let indices: Vec<u32> = idx_u64s.into_iter().map(|n| n as u32).collect();

        let tri_count = slot_with(handle, |portal| {
            portal.upload_mesh(&positions, &indices)
        })
        .ok_or_else(|| args::bad(span, "gpu_upload_mesh: invalid handle"))?;

        Ok(args::record([
            ("triangle_count", Value::U64(tri_count as u64)),
        ]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (args, span);
        Err(args::bad(span, "gpu_upload_mesh requires native build"))
    }
}

/// `Render.gpu_upload_tensor` — upload a tensor node buffer (raw bytes).
pub fn gpu_upload_tensor(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = args::rec_u64(args, "handle").ok_or_else(|| {
            args::bad(span, "gpu_upload_tensor needs { handle: u64 }")
        })?;

        let bytes = args::rec(args, "buffer")
            .and_then(|v| args::u8s(v))
            .ok_or_else(|| args::bad(span, "gpu_upload_tensor needs { buffer: [u8] }"))?;

        let node_count = slot_with(handle, |portal| {
            portal.upload_tensor_buffer(&bytes)
        })
        .ok_or_else(|| args::bad(span, "gpu_upload_tensor: invalid handle"))?
        .map_err(|e| args::bad(span, format!("gpu_upload_tensor: {e}")))?;

        Ok(args::record([
            ("node_count", Value::U64(node_count as u64)),
        ]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (args, span);
        Err(args::bad(span, "gpu_upload_tensor requires native build"))
    }
}

/// `Render.gpu_set_camera` — set camera orbit angles.
pub fn gpu_set_camera(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = args::rec_u64(args, "handle").ok_or_else(|| {
            args::bad(span, "gpu_set_camera needs { handle: u64 }")
        })?;
        let yaw = args::rec_f64(args, "yaw").unwrap_or(0.0) as f32;
        let pitch = args::rec_f64(args, "pitch").unwrap_or(0.0) as f32;
        let zoom = args::rec_f64(args, "zoom").unwrap_or(1.0) as f32;

        slot_with(handle, |portal| {
            portal.set_camera(yaw, pitch, zoom);
        })
        .ok_or_else(|| args::bad(span, "gpu_set_camera: invalid handle"))?;

        Ok(Value::Record(BTreeMap::new()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (args, span);
        Err(args::bad(span, "gpu_set_camera requires native build"))
    }
}

/// `Render.gpu_pick` — queue a pick query at screen coordinates.
pub fn gpu_pick(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = args::rec_u64(args, "handle").ok_or_else(|| {
            args::bad(span, "gpu_pick needs { handle: u64 }")
        })?;
        let x = args::rec_f64(args, "x").unwrap_or(0.0) as f32;
        let y = args::rec_f64(args, "y").unwrap_or(0.0) as f32;

        slot_with(handle, |portal| {
            portal.queue_pick(x, y);
        })
        .ok_or_else(|| args::bad(span, "gpu_pick: invalid handle"))?;

        Ok(Value::Record(BTreeMap::new()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (args, span);
        Err(args::bad(span, "gpu_pick requires native build"))
    }
}

/// `Render.gpu_poll_pick` — poll for a completed pick result.
pub fn gpu_poll_pick(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = args::rec_u64(args, "handle").ok_or_else(|| {
            args::bad(span, "gpu_poll_pick needs { handle: u64 }")
        })?;

        let result = slot_with(handle, |portal| {
            portal.poll_pick_readback()
        })
        .ok_or_else(|| args::bad(span, "gpu_poll_pick: invalid handle"))?;

        match result {
            Some(node_id) => Ok(args::record([
                ("found", Value::Bool(true)),
                ("node_id", Value::U64(node_id as u64)),
            ])),
            None => Ok(args::record([
                ("found", Value::Bool(false)),
            ])),
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (args, span);
        Err(args::bad(span, "gpu_poll_pick requires native build"))
    }
}

/// `Render.gpu_resize` — resize the viewport.
pub fn gpu_resize(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = args::rec_u64(args, "handle").ok_or_else(|| {
            args::bad(span, "gpu_resize needs { handle: u64 }")
        })?;
        let width = args::rec_u64(args, "width").unwrap_or(800) as u32;
        let height = args::rec_u64(args, "height").unwrap_or(600) as u32;

        slot_with(handle, |portal| {
            portal.resize(width, height);
        })
        .ok_or_else(|| args::bad(span, "gpu_resize: invalid handle"))?;

        Ok(Value::Record(BTreeMap::new()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (args, span);
        Err(args::bad(span, "gpu_resize requires native build"))
    }
}

/// `Render.gpu_set_ambient` — enable/disable the ambient particle field.
pub fn gpu_set_ambient(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = args::rec_u64(args, "handle").ok_or_else(|| {
            args::bad(span, "gpu_set_ambient needs { handle: u64 }")
        })?;
        let enabled = args::rec_bool(args, "enabled").unwrap_or(true);

        slot_with(handle, |portal| {
            portal.set_ambient_enabled(enabled);
        })
        .ok_or_else(|| args::bad(span, "gpu_set_ambient: invalid handle"))?;

        Ok(Value::Record(BTreeMap::new()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (args, span);
        Err(args::bad(span, "gpu_set_ambient requires native build"))
    }
}

/// `Render.gpu_destroy` — destroy a PortalGpu instance and free its slot.
pub fn gpu_destroy(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = args::rec_u64(args, "handle").ok_or_else(|| {
            args::bad(span, "gpu_destroy needs { handle: u64 }")
        })?;

        if slot_remove(handle) {
            Ok(args::record([("destroyed", Value::Bool(true))]))
        } else {
            Err(args::bad(span, "gpu_destroy: invalid handle"))
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (args, span);
        Err(args::bad(span, "gpu_destroy requires native build"))
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;

    fn snap() -> crate::poet_host::PoetSnapshot {
        crate::poet_host::PoetSnapshot::default()
    }

    fn eval(src: &str) -> Value {
        let mut snap = snap();
        snap.eval_fn(src, "go", vec![]).expect("script should eval")
    }

    fn assert_record(v: &Value) {
        assert!(matches!(v, Value::Record(_)), "expected record, got {v:?}");
    }

    fn record_get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
        match v {
            Value::Record(m) => m.get(key),
            _ => None,
        }
    }

    fn dummy_span() -> Span {
        Span::new(0, 0)
    }

    #[test]
    fn g_gpu_adapter_info_returns_record() {
        let src = r#"
        requires [ capability("capability.invoke") ];
        effect fn go() {
            return capability.invoke("Render.gpu_adapter_info", {});
        }
        "#;
        let result = eval(src);
        assert_record(&result);
    }

    #[test]
    fn g_gpu_init_and_render_cycle() {
        let ctx = gpu_context::try_shared_gpu();
        if ctx.is_none() {
            eprintln!("[gpu_init_test] no GPU adapter — skipping");
            return;
        }

        let span = dummy_span();
        let init_args = args::record([
            ("width", Value::U64(64)),
            ("height", Value::U64(64)),
            ("particle_cap", Value::U64(256)),
        ]);
        let init_result = gpu_init(&init_args, span).expect("gpu_init should succeed");
        let handle = record_get(&init_result, "handle").expect("has handle");
        let Value::U64(h) = handle else { panic!("handle not u64") };

        let render_args = args::record([
            ("handle", Value::U64(*h)),
            ("time", Value::F64(0.0)),
        ]);
        let render_result = gpu_render_frame(&render_args, span).expect("gpu_render_frame should succeed");
        assert_record(&render_result);

        let destroy_args = args::record([("handle", Value::U64(*h))]);
        gpu_destroy(&destroy_args, span).expect("gpu_destroy should succeed");
    }

    #[test]
    fn g_gpu_set_camera() {
        let ctx = gpu_context::try_shared_gpu();
        if ctx.is_none() {
            eprintln!("[gpu_camera_test] no GPU adapter — skipping");
            return;
        }

        let span = dummy_span();
        let init_args = args::record([
            ("width", Value::U64(128)),
            ("height", Value::U64(128)),
        ]);
        let init_result = gpu_init(&init_args, span).expect("gpu_init should succeed");
        let Value::U64(h) = record_get(&init_result, "handle").unwrap() else { panic!() };

        let cam_args = args::record([
            ("handle", Value::U64(*h)),
            ("yaw", Value::F64(0.5)),
            ("pitch", Value::F64(-0.3)),
            ("zoom", Value::F64(1.5)),
        ]);
        gpu_set_camera(&cam_args, span).expect("gpu_set_camera should succeed");

        let render_args = args::record([
            ("handle", Value::U64(*h)),
            ("time", Value::F64(1.0)),
        ]);
        gpu_render_frame(&render_args, span).expect("gpu_render_frame should succeed");

        let destroy_args = args::record([("handle", Value::U64(*h))]);
        gpu_destroy(&destroy_args, span).expect("gpu_destroy should succeed");
    }

    #[test]
    fn g_gpu_resize_and_ambient() {
        let ctx = gpu_context::try_shared_gpu();
        if ctx.is_none() {
            eprintln!("[gpu_resize_test] no GPU adapter — skipping");
            return;
        }

        let span = dummy_span();
        let init_args = args::record([
            ("width", Value::U64(64)),
            ("height", Value::U64(64)),
        ]);
        let init_result = gpu_init(&init_args, span).expect("gpu_init should succeed");
        let Value::U64(h) = record_get(&init_result, "handle").unwrap() else { panic!() };

        let resize_args = args::record([
            ("handle", Value::U64(*h)),
            ("width", Value::U64(128)),
            ("height", Value::U64(128)),
        ]);
        gpu_resize(&resize_args, span).expect("gpu_resize should succeed");

        let ambient_args = args::record([
            ("handle", Value::U64(*h)),
            ("enabled", Value::Bool(true)),
        ]);
        gpu_set_ambient(&ambient_args, span).expect("gpu_set_ambient should succeed");

        let render_args = args::record([
            ("handle", Value::U64(*h)),
            ("time", Value::F64(0.0)),
        ]);
        gpu_render_frame(&render_args, span).expect("gpu_render_frame should succeed");

        let destroy_args = args::record([("handle", Value::U64(*h))]);
        gpu_destroy(&destroy_args, span).expect("gpu_destroy should succeed");
    }

    #[test]
    fn g_gpu_pick_readback_alignment() {
        let ctx = gpu_context::try_shared_gpu();
        if ctx.is_none() {
            eprintln!("[gpu_pick_test] no GPU adapter — skipping");
            return;
        }

        let span = dummy_span();
        let init_args = args::record([
            ("width", Value::U64(128)),
            ("height", Value::U64(128)),
        ]);
        let init_result = gpu_init(&init_args, span).expect("gpu_init should succeed");
        let Value::U64(h) = record_get(&init_result, "handle").unwrap() else { panic!() };

        let pick_args = args::record([
            ("handle", Value::U64(*h)),
            ("x", Value::F64(64.0)),
            ("y", Value::F64(64.0)),
        ]);
        gpu_pick(&pick_args, span).expect("gpu_pick should succeed");

        let render_args = args::record([
            ("handle", Value::U64(*h)),
            ("time", Value::F64(0.0)),
        ]);
        gpu_render_frame(&render_args, span).expect("gpu_render_frame should succeed");

        let poll_args = args::record([("handle", Value::U64(*h))]);
        let poll_result = gpu_poll_pick(&poll_args, span).expect("gpu_poll_pick should succeed");
        assert_record(&poll_result);

        let destroy_args = args::record([("handle", Value::U64(*h))]);
        gpu_destroy(&destroy_args, span).expect("gpu_destroy should succeed");
    }
}
