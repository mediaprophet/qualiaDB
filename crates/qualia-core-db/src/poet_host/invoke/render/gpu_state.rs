//! Remaining PortalGpu invoke surface — camera/standpoint/artefact/mesh queries.
//!
//! Completes the WebGPU capability table so Vibe is not limited to the MVP
//! init/render/upload/pick subset. Every public `PortalGpu` method that is
//! meaningful from a script (not device/queue internals) has an invoke id.

use super::super::args;
#[cfg(not(target_arch = "wasm32"))]
use super::gpu::slot_with;
use vibe::{Diagnostic, Span, Value};

#[cfg(not(target_arch = "wasm32"))]
use crate::render::physics::{Aabb, Joint};
#[cfg(not(target_arch = "wasm32"))]
use crate::render::telemetry::{
    ObserverStandpoint, DEONTIC_LANE_COMMONS, FABRIC_VIEWPORT_LOCAL, STANDPOINT_SPECTATOR,
};

#[cfg(target_arch = "wasm32")]
fn native_only(span: Span, name: &str) -> Result<Value, Diagnostic> {
    Err(args::bad(span, format!("{name} requires native build")))
}

#[cfg(not(target_arch = "wasm32"))]
fn handle_of(args: &Value, span: Span, name: &str) -> Result<u64, Diagnostic> {
    args::rec_u64(args, "handle")
        .ok_or_else(|| args::bad(span, format!("{name} needs {{ handle: u64 }}")))
}

#[cfg(not(target_arch = "wasm32"))]
fn vec3(args: &Value, key: &str) -> Option<[f32; 3]> {
    let xs = args::rec_f64_list(args, key)?;
    if xs.len() != 3 {
        return None;
    }
    Some([xs[0] as f32, xs[1] as f32, xs[2] as f32])
}

#[cfg(not(target_arch = "wasm32"))]
fn class_from(s: &str) -> u32 {
    match s {
        "ephemeral" => 1,
        "did" => 2,
        "vault" => 3,
        _ => 0,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn class_name(c: u32) -> &'static str {
    match c {
        1 => "ephemeral",
        2 => "did",
        3 => "vault",
        _ => "spectator",
    }
}

/// `Render.gpu_upload_mesh_colored` — triangle mesh with per-vertex RGBA.
pub fn gpu_upload_mesh_colored(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args = _args;
        let handle = handle_of(args, span, "gpu_upload_mesh_colored")?;
        let pos_f64s = args::rec_f64_list(args, "positions")
            .ok_or_else(|| args::bad(span, "gpu_upload_mesh_colored needs { positions: [f32] }"))?;
        let col_f64s = args::rec_f64_list(args, "colors").unwrap_or_default();
        let idx_u64s = args::rec_u64_list(args, "indices")
            .ok_or_else(|| args::bad(span, "gpu_upload_mesh_colored needs { indices: [u32] }"))?;

        let positions: Vec<[f32; 3]> = pos_f64s
            .chunks_exact(3)
            .map(|c| [c[0] as f32, c[1] as f32, c[2] as f32])
            .collect();
        let colors: Vec<[f32; 4]> = col_f64s
            .chunks_exact(4)
            .map(|c| [c[0] as f32, c[1] as f32, c[2] as f32, c[3] as f32])
            .collect();
        let indices: Vec<u32> = idx_u64s.into_iter().map(|n| n as u32).collect();

        if !colors.is_empty() && colors.len() != positions.len() {
            return Err(args::bad(
                span,
                "gpu_upload_mesh_colored: colors length must match vertex count",
            ));
        }

        let tri_count = slot_with(handle, |portal| {
            portal.upload_mesh_colored(&positions, &colors, &indices)
        })
        .ok_or_else(|| args::bad(span, "gpu_upload_mesh_colored: invalid handle"))?;

        Ok(args::record([(
            "triangle_count",
            Value::U64(tri_count as u64),
        )]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        native_only(span, "gpu_upload_mesh_colored")
    }
}

/// `Render.gpu_set_standpoint` — human-centric observer standpoint.
pub fn gpu_set_standpoint(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args = _args;
        let handle = handle_of(args, span, "gpu_set_standpoint")?;
        let class = args::rec_str(args, "class")
            .map(class_from)
            .or_else(|| args::rec_u64(args, "standpoint_class").map(|n| n as u32))
            .unwrap_or(STANDPOINT_SPECTATOR);
        let observer = ObserverStandpoint::new(
            args::rec_u64(args, "standpoint_hash").unwrap_or(0),
            args::rec_u64(args, "session_nonce").unwrap_or(0),
            class,
            args::rec_f64(args, "epistemic_q").unwrap_or(1.0) as f32,
            args::rec_f64(args, "t_slice").unwrap_or(0.5) as f32,
            args::rec_f64(args, "t_window").unwrap_or(0.1) as f32,
            args::rec_u64(args, "deontic_lane").unwrap_or(DEONTIC_LANE_COMMONS as u64) as u32,
            args::rec_u64(args, "fabric_gate").unwrap_or(FABRIC_VIEWPORT_LOCAL as u64) as u32,
        );
        slot_with(handle, |portal| portal.set_standpoint(observer))
            .ok_or_else(|| args::bad(span, "gpu_set_standpoint: invalid handle"))?;
        Ok(args::record([("set", Value::Bool(true))]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        native_only(span, "gpu_set_standpoint")
    }
}

/// `Render.gpu_observer_standpoint` — read the current observer standpoint.
pub fn gpu_observer_standpoint(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args = _args;
        let handle = handle_of(args, span, "gpu_observer_standpoint")?;
        let o = slot_with(handle, |portal| portal.observer_standpoint())
            .ok_or_else(|| args::bad(span, "gpu_observer_standpoint: invalid handle"))?;
        Ok(args::record([
            ("standpoint_hash", Value::U64(o.standpoint_hash)),
            ("session_nonce", Value::U64(o.session_nonce)),
            ("epistemic_q", Value::F64(o.epistemic_q as f64)),
            ("t_slice", Value::F64(o.t_slice as f64)),
            ("t_window", Value::F64(o.t_window as f64)),
            ("deontic_lane", Value::U64(o.deontic_lane as u64)),
            ("standpoint_class", Value::U64(o.standpoint_class as u64)),
            (
                "class",
                Value::String(class_name(o.standpoint_class).into()),
            ),
            ("fabric_gate", Value::U64(o.fabric_gate as u64)),
        ]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        native_only(span, "gpu_observer_standpoint")
    }
}

/// `Render.gpu_camera_state` — read yaw/pitch/zoom.
pub fn gpu_camera_state(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args = _args;
        let handle = handle_of(args, span, "gpu_camera_state")?;
        let cam = slot_with(handle, |portal| portal.camera_state())
            .ok_or_else(|| args::bad(span, "gpu_camera_state: invalid handle"))?;
        Ok(args::record([
            ("yaw", Value::F64(cam.yaw as f64)),
            ("pitch", Value::F64(cam.pitch as f64)),
            ("zoom", Value::F64(cam.zoom as f64)),
        ]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        native_only(span, "gpu_camera_state")
    }
}

/// `Render.gpu_surface_size` — configured color/depth extent.
pub fn gpu_surface_size(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args = _args;
        let handle = handle_of(args, span, "gpu_surface_size")?;
        let (w, h) = slot_with(handle, |portal| portal.surface_size())
            .ok_or_else(|| args::bad(span, "gpu_surface_size: invalid handle"))?;
        Ok(args::record([
            ("width", Value::U64(w as u64)),
            ("height", Value::U64(h as u64)),
        ]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        native_only(span, "gpu_surface_size")
    }
}

/// `Render.gpu_has_mesh`
pub fn gpu_has_mesh(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args = _args;
        let handle = handle_of(args, span, "gpu_has_mesh")?;
        let has = slot_with(handle, |portal| portal.has_mesh())
            .ok_or_else(|| args::bad(span, "gpu_has_mesh: invalid handle"))?;
        Ok(args::record([("has_mesh", Value::Bool(has))]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        native_only(span, "gpu_has_mesh")
    }
}

/// `Render.gpu_has_tensor`
pub fn gpu_has_tensor(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args = _args;
        let handle = handle_of(args, span, "gpu_has_tensor")?;
        let has = slot_with(handle, |portal| portal.has_tensor_buffer())
            .ok_or_else(|| args::bad(span, "gpu_has_tensor: invalid handle"))?;
        Ok(args::record([("has_tensor", Value::Bool(has))]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        native_only(span, "gpu_has_tensor")
    }
}

/// `Render.gpu_tensor_node_count`
pub fn gpu_tensor_node_count(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args = _args;
        let handle = handle_of(args, span, "gpu_tensor_node_count")?;
        let n = slot_with(handle, |portal| portal.tensor_node_count())
            .ok_or_else(|| args::bad(span, "gpu_tensor_node_count: invalid handle"))?;
        Ok(args::record([("node_count", Value::U64(n as u64))]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        native_only(span, "gpu_tensor_node_count")
    }
}

/// `Render.gpu_particle_count`
pub fn gpu_particle_count(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args = _args;
        let handle = handle_of(args, span, "gpu_particle_count")?;
        let n = slot_with(handle, |portal| portal.particle_count())
            .ok_or_else(|| args::bad(span, "gpu_particle_count: invalid handle"))?;
        Ok(args::record([("particle_count", Value::U64(n as u64))]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        native_only(span, "gpu_particle_count")
    }
}

/// `Render.gpu_sync_bloom` — reconcile HDR bloom targets with VRAM mode.
pub fn gpu_sync_bloom(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args = _args;
        let handle = handle_of(args, span, "gpu_sync_bloom")?;
        slot_with(handle, |portal| portal.sync_bloom_targets())
            .ok_or_else(|| args::bad(span, "gpu_sync_bloom: invalid handle"))?;
        Ok(args::record([("synced", Value::Bool(true))]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        native_only(span, "gpu_sync_bloom")
    }
}

/// `Render.gpu_set_artefact_joint` — kinematic joint (revolute/prismatic) or clear.
pub fn gpu_set_artefact_joint(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args = _args;
        let handle = handle_of(args, span, "gpu_set_artefact_joint")?;
        let clear = args::rec_bool(args, "clear").unwrap_or(false);
        let joint = if clear {
            None
        } else {
            let kind = args::rec_str(args, "kind").unwrap_or("revolute");
            let axis = vec3(args, "axis").unwrap_or([0.0, 1.0, 0.0]);
            let rate = args::rec_f64(args, "rate").unwrap_or(1.0) as f32;
            Some(match kind {
                "prismatic" => Joint::prismatic(axis, rate),
                _ => Joint::revolute(axis, rate),
            })
        };
        slot_with(handle, |portal| portal.set_artefact_joint(joint))
            .ok_or_else(|| args::bad(span, "gpu_set_artefact_joint: invalid handle"))?;
        Ok(args::record([("set", Value::Bool(true))]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        native_only(span, "gpu_set_artefact_joint")
    }
}

/// `Render.gpu_set_artefact_world` — world AABB constraint, or clear.
pub fn gpu_set_artefact_world(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args = _args;
        let handle = handle_of(args, span, "gpu_set_artefact_world")?;
        let clear = args::rec_bool(args, "clear").unwrap_or(false);
        let world = if clear {
            None
        } else {
            let min = vec3(args, "min").unwrap_or([-1.0, -1.0, -1.0]);
            let max = vec3(args, "max").unwrap_or([1.0, 1.0, 1.0]);
            Some(Aabb::new(min, max))
        };
        slot_with(handle, |portal| portal.set_artefact_world(world))
            .ok_or_else(|| args::bad(span, "gpu_set_artefact_world: invalid handle"))?;
        Ok(args::record([("set", Value::Bool(true))]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        native_only(span, "gpu_set_artefact_world")
    }
}

/// `Render.gpu_artefact_refused` — last frame's joint pose was refused.
pub fn gpu_artefact_refused(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args = _args;
        let handle = handle_of(args, span, "gpu_artefact_refused")?;
        let refused = slot_with(handle, |portal| portal.artefact_refused())
            .ok_or_else(|| args::bad(span, "gpu_artefact_refused: invalid handle"))?;
        Ok(args::record([("refused", Value::Bool(refused))]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        native_only(span, "gpu_artefact_refused")
    }
}

/// `Render.gpu_required_rgba8_bytes` — readback buffer size for current surface.
pub fn gpu_required_rgba8_bytes(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args = _args;
        let handle = handle_of(args, span, "gpu_required_rgba8_bytes")?;
        let n = slot_with(handle, |portal| portal.required_rgba8_bytes())
            .ok_or_else(|| args::bad(span, "gpu_required_rgba8_bytes: invalid handle"))?;
        Ok(args::record([("bytes", Value::U64(n as u64))]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        native_only(span, "gpu_required_rgba8_bytes")
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;
    use crate::gpu_context;
    use crate::poet_host::invoke::render::gpu::{
        gpu_destroy, gpu_init, gpu_set_camera, gpu_upload_mesh,
    };

    fn dummy_span() -> Span {
        Span::new(0, 0)
    }

    fn rec_u64(v: &Value, key: &str) -> u64 {
        match v {
            Value::Record(m) => match m.get(key) {
                Some(Value::U64(n)) => *n,
                _ => panic!("missing u64 {key}"),
            },
            _ => panic!("expected record"),
        }
    }

    fn rec_bool(v: &Value, key: &str) -> bool {
        match v {
            Value::Record(m) => match m.get(key) {
                Some(Value::Bool(b)) => *b,
                _ => panic!("missing bool {key}"),
            },
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn g_gpu_state_surface_is_comprehensive() {
        if gpu_context::try_shared_gpu().is_none() {
            eprintln!("[gpu_state] no GPU adapter — skipping");
            return;
        }
        let span = dummy_span();
        let init = gpu_init(
            &args::record([("width", Value::U64(64)), ("height", Value::U64(64))]),
            span,
        )
        .expect("init");
        let h = rec_u64(&init, "handle");

        let size = gpu_surface_size(&args::record([("handle", Value::U64(h))]), span).unwrap();
        assert_eq!(rec_u64(&size, "width"), 64);
        assert_eq!(rec_u64(&size, "height"), 64);

        gpu_set_camera(
            &args::record([
                ("handle", Value::U64(h)),
                ("yaw", Value::F64(0.25)),
                ("pitch", Value::F64(-0.1)),
                ("zoom", Value::F64(4.0)),
            ]),
            span,
        )
        .unwrap();
        let cam = gpu_camera_state(&args::record([("handle", Value::U64(h))]), span).unwrap();
        match cam {
            Value::Record(m) => {
                assert!((m.get("yaw").and_then(Value::as_f64).unwrap() - 0.25).abs() < 1e-5);
            }
            _ => panic!("camera record"),
        }

        gpu_set_standpoint(
            &args::record([
                ("handle", Value::U64(h)),
                ("class", Value::String("did".into())),
                ("epistemic_q", Value::F64(0.8)),
            ]),
            span,
        )
        .unwrap();
        let obs =
            gpu_observer_standpoint(&args::record([("handle", Value::U64(h))]), span).unwrap();
        match obs {
            Value::Record(m) => {
                assert_eq!(m.get("class"), Some(&Value::String("did".into())));
            }
            _ => panic!("observer record"),
        }

        let mesh = gpu_upload_mesh(
            &args::record([
                ("handle", Value::U64(h)),
                (
                    "positions",
                    Value::List(vec![
                        Value::F64(0.0),
                        Value::F64(0.0),
                        Value::F64(0.0),
                        Value::F64(1.0),
                        Value::F64(0.0),
                        Value::F64(0.0),
                        Value::F64(0.0),
                        Value::F64(1.0),
                        Value::F64(0.0),
                    ]),
                ),
                (
                    "indices",
                    Value::List(vec![Value::U64(0), Value::U64(1), Value::U64(2)]),
                ),
            ]),
            span,
        )
        .unwrap();
        assert_eq!(rec_u64(&mesh, "triangle_count"), 1);
        assert!(rec_bool(
            &gpu_has_mesh(&args::record([("handle", Value::U64(h))]), span).unwrap(),
            "has_mesh"
        ));
        assert!(!rec_bool(
            &gpu_has_tensor(&args::record([("handle", Value::U64(h))]), span).unwrap(),
            "has_tensor"
        ));

        gpu_set_artefact_joint(
            &args::record([
                ("handle", Value::U64(h)),
                ("kind", Value::String("revolute".into())),
                (
                    "axis",
                    Value::List(vec![Value::F64(0.0), Value::F64(1.0), Value::F64(0.0)]),
                ),
                ("rate", Value::F64(0.5)),
            ]),
            span,
        )
        .unwrap();
        gpu_set_artefact_world(
            &args::record([
                ("handle", Value::U64(h)),
                (
                    "min",
                    Value::List(vec![Value::F64(-2.0), Value::F64(-2.0), Value::F64(-2.0)]),
                ),
                (
                    "max",
                    Value::List(vec![Value::F64(2.0), Value::F64(2.0), Value::F64(2.0)]),
                ),
            ]),
            span,
        )
        .unwrap();
        gpu_sync_bloom(&args::record([("handle", Value::U64(h))]), span).unwrap();
        let bytes =
            gpu_required_rgba8_bytes(&args::record([("handle", Value::U64(h))]), span).unwrap();
        assert_eq!(rec_u64(&bytes, "bytes"), 64 * 64 * 4);

        let colored = gpu_upload_mesh_colored(
            &args::record([
                ("handle", Value::U64(h)),
                (
                    "positions",
                    Value::List(vec![
                        Value::F64(0.0),
                        Value::F64(0.0),
                        Value::F64(0.0),
                        Value::F64(1.0),
                        Value::F64(0.0),
                        Value::F64(0.0),
                        Value::F64(0.0),
                        Value::F64(1.0),
                        Value::F64(0.0),
                    ]),
                ),
                (
                    "colors",
                    Value::List(vec![
                        Value::F64(1.0),
                        Value::F64(0.0),
                        Value::F64(0.0),
                        Value::F64(1.0),
                        Value::F64(0.0),
                        Value::F64(1.0),
                        Value::F64(0.0),
                        Value::F64(1.0),
                        Value::F64(0.0),
                        Value::F64(0.0),
                        Value::F64(1.0),
                        Value::F64(1.0),
                    ]),
                ),
                (
                    "indices",
                    Value::List(vec![Value::U64(0), Value::U64(1), Value::U64(2)]),
                ),
            ]),
            span,
        )
        .unwrap();
        assert_eq!(rec_u64(&colored, "triangle_count"), 1);

        gpu_destroy(&args::record([("handle", Value::U64(h))]), span).unwrap();
    }
}
