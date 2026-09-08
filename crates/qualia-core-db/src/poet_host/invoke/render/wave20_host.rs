//! Wave-20 Host binds: Scene camera numerics.
//!
//! Pure CPU paths from `render::navigation` — no forge / `caps()` / CUDA / GPU.

use super::super::args;
use crate::render::camera::CameraState;
use crate::render::navigation::{camera_frame_node, lerp_camera};
use vibe::{Diagnostic, Span, Value};

fn read_camera(args_v: &Value, key: &str, span: Span, what: &str) -> Result<CameraState, Diagnostic> {
    let rec = args::rec(args_v, key).ok_or_else(|| {
        args::bad(span, format!("{what} needs {key}: {{ yaw, pitch, zoom }}"))
    })?;
    let yaw = args::rec_f64(rec, "yaw").unwrap_or(0.0) as f32;
    let pitch = args::rec_f64(rec, "pitch").unwrap_or(0.0) as f32;
    let zoom = args::rec_f64(rec, "zoom").unwrap_or(3.5) as f32;
    Ok(CameraState { yaw, pitch, zoom })
}

fn camera_record(cam: CameraState) -> Value {
    args::record([
        ("yaw", Value::F64(cam.yaw as f64)),
        ("pitch", Value::F64(cam.pitch as f64)),
        ("zoom", Value::F64(cam.zoom as f64)),
    ])
}

/// `Scene.lerp_camera` — interpolate orbit camera states.
/// Args: `{ a: {yaw,pitch,zoom}, b: {yaw,pitch,zoom}, t: f64 }`.
/// Out: `{ yaw, pitch, zoom }`.
pub fn lerp_camera_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = read_camera(args_v, "a", span, "Scene.lerp_camera")?;
    let b = read_camera(args_v, "b", span, "Scene.lerp_camera")?;
    let t = args::rec_f64(args_v, "t")
        .ok_or_else(|| args::bad(span, "Scene.lerp_camera needs t"))? as f32;
    Ok(camera_record(lerp_camera(a, b, t)))
}

/// `Scene.camera_frame_node` — orbit params that frame a world-space node.
/// Args: `{ node: [f64;3] }`. Out: `{ yaw, pitch, zoom }`.
pub fn camera_frame_node_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let node = args::rec_f64_list(args_v, "node")
        .ok_or_else(|| args::bad(span, "Scene.camera_frame_node needs node: [f64;3]"))?;
    if node.len() < 3 {
        return Err(args::bad(
            span,
            "Scene.camera_frame_node: node needs ≥3 coords",
        ));
    }
    let cam = camera_frame_node([node[0] as f32, node[1] as f32, node[2] as f32]);
    Ok(camera_record(cam))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn cam(yaw: f64, pitch: f64, zoom: f64) -> Value {
        let mut m = BTreeMap::new();
        m.insert("yaw".into(), Value::F64(yaw));
        m.insert("pitch".into(), Value::F64(pitch));
        m.insert("zoom".into(), Value::F64(zoom));
        Value::Record(m)
    }

    #[test]
    fn wave20_lerp_camera_midpoint() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), cam(0.0, 0.0, 1.0));
        m.insert("b".into(), cam(2.0, 0.4, 5.0));
        m.insert("t".into(), Value::F64(0.5));
        let out = lerp_camera_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "yaw").unwrap() - 1.0).abs() < 1e-5);
        assert!((args::rec_f64(&out, "pitch").unwrap() - 0.2).abs() < 1e-5);
        assert!((args::rec_f64(&out, "zoom").unwrap() - 3.0).abs() < 1e-5);
    }

    #[test]
    fn wave20_camera_frame_node_finite() {
        let mut m = BTreeMap::new();
        m.insert("node".into(), args::f64_list_value([1.0, 0.5, -2.0]));
        let out = camera_frame_node_host(&Value::Record(m), span()).unwrap();
        assert!(args::rec_f64(&out, "yaw").unwrap().is_finite());
        assert!(args::rec_f64(&out, "pitch").unwrap().is_finite());
        assert!(args::rec_f64(&out, "zoom").unwrap().is_finite());
    }
}
