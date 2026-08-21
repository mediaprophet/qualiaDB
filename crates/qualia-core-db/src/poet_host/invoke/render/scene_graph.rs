//! Scene graph invoke extensions — high-level scene operations wrapping
//! the existing render infrastructure.

use super::super::args;
use poet_vibe::{DiagCode, Diagnostic, Span, Value};

/// `Scene.create` — create a new scene by name. Returns a scene handle
/// record. Wraps the existing `Render.scene` infrastructure.
pub fn scene_create(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let name =
        args::rec_str(args, "name").ok_or_else(|| args::bad(span, "Scene.create needs name"))?;
    Ok(args::record([
        ("name", Value::String(name.to_string())),
        ("nodes", Value::List(vec![])),
        ("edges", Value::List(vec![])),
        ("faces", Value::List(vec![])),
        ("status", Value::String("created".into())),
    ]))
}

/// `Scene.add_node` — add a node to a scene graph. Returns the node record.
pub fn scene_add_node(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_u64(args, "id").ok_or_else(|| args::bad(span, "Scene.add_node needs id"))?;
    let x = args::rec_f64(args, "x").unwrap_or(0.0);
    let y = args::rec_f64(args, "y").unwrap_or(0.0);
    let z = args::rec_f64(args, "z").unwrap_or(0.0);
    Ok(args::record([
        ("id", Value::U64(id)),
        ("x", Value::F64(x)),
        ("y", Value::F64(y)),
        ("z", Value::F64(z)),
        ("status", Value::String("added".into())),
    ]))
}

/// `Scene.set_transform` — set a node's transform (position, rotation, scale).
pub fn scene_set_transform(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let node_id = args::rec_u64(args, "node_id")
        .ok_or_else(|| args::bad(span, "Scene.set_transform needs node_id"))?;
    let tx = args::rec_f64(args, "tx").unwrap_or(0.0);
    let ty = args::rec_f64(args, "ty").unwrap_or(0.0);
    let tz = args::rec_f64(args, "tz").unwrap_or(0.0);
    let rx = args::rec_f64(args, "rx").unwrap_or(0.0);
    let ry = args::rec_f64(args, "ry").unwrap_or(0.0);
    let rz = args::rec_f64(args, "rz").unwrap_or(0.0);
    let sx = args::rec_f64(args, "sx").unwrap_or(1.0);
    let sy = args::rec_f64(args, "sy").unwrap_or(1.0);
    let sz = args::rec_f64(args, "sz").unwrap_or(1.0);
    Ok(args::record([
        ("node_id", Value::U64(node_id)),
        ("tx", Value::F64(tx)),
        ("ty", Value::F64(ty)),
        ("tz", Value::F64(tz)),
        ("rx", Value::F64(rx)),
        ("ry", Value::F64(ry)),
        ("rz", Value::F64(rz)),
        ("sx", Value::F64(sx)),
        ("sy", Value::F64(sy)),
        ("sz", Value::F64(sz)),
        ("status", Value::String("transformed".into())),
    ]))
}

/// `Scene.set_mesh` — assign a mesh to a node. The mesh data is uploaded
/// via `Render.gpu_upload_mesh` separately; this records the assignment.
pub fn scene_set_mesh(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let node_id = args::rec_u64(args, "node_id")
        .ok_or_else(|| args::bad(span, "Scene.set_mesh needs node_id"))?;
    let mesh_iri = args::rec_str(args, "mesh_iri")
        .ok_or_else(|| args::bad(span, "Scene.set_mesh needs mesh_iri"))?;
    Ok(args::record([
        ("node_id", Value::U64(node_id)),
        ("mesh_iri", Value::String(mesh_iri.to_string())),
        ("status", Value::String("mesh_assigned".into())),
    ]))
}

/// `Scene.add_camera` — add a camera to a scene.
pub fn scene_add_camera(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").unwrap_or(0.0);
    let y = args::rec_f64(args, "y").unwrap_or(0.0);
    let z = args::rec_f64(args, "z").unwrap_or(5.0);
    let fov = args::rec_f64(args, "fov").unwrap_or(60.0);
    Ok(args::record([
        ("x", Value::F64(x)),
        ("y", Value::F64(y)),
        ("z", Value::F64(z)),
        ("fov", Value::F64(fov)),
        ("status", Value::String("camera_added".into())),
    ]))
}

/// `Scene.render` — high-level render request. Wraps `Render.gpu_render_frame`.
/// The actual rendering is done by the GPU invoke; this is a planning record.
pub fn scene_render(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let scene_name =
        args::rec_str(args, "scene").ok_or_else(|| args::bad(span, "Scene.render needs scene"))?;
    let camera_id = args::rec_u64(args, "camera_id").unwrap_or(0);
    Ok(args::record([
        ("scene", Value::String(scene_name.to_string())),
        ("camera_id", Value::U64(camera_id)),
        ("status", Value::String("render_requested".into())),
        ("backend", Value::String("webgpu_or_webgl2".into())),
    ]))
}

/// `Scene.set_viewport` — configure viewport resolution and format.
pub fn scene_set_viewport(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let width = args::rec_u64(args, "width")
        .ok_or_else(|| args::bad(span, "Scene.set_viewport needs width"))?;
    let height = args::rec_u64(args, "height")
        .ok_or_else(|| args::bad(span, "Scene.set_viewport needs height"))?;
    let format = args::rec_str(args, "format").unwrap_or("rgba8unorm");
    Ok(args::record([
        ("width", Value::U64(width)),
        ("height", Value::U64(height)),
        ("format", Value::String(format.to_string())),
        ("status", Value::String("viewport_set".into())),
    ]))
}

/// `Scene.set_clear_colour` — set the viewport clear colour.
pub fn scene_set_clear_colour(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let r = args::rec_f64(args, "r").unwrap_or(0.0);
    let g = args::rec_f64(args, "g").unwrap_or(0.0);
    let b = args::rec_f64(args, "b").unwrap_or(0.0);
    let a = args::rec_f64(args, "a").unwrap_or(1.0);
    Ok(args::record([
        ("r", Value::F64(r)),
        ("g", Value::F64(g)),
        ("b", Value::F64(b)),
        ("a", Value::F64(a)),
        ("status", Value::String("clear_colour_set".into())),
    ]))
}

/// `Scene.capture_frame` — request a frame capture. The actual pixel
/// readback is done by `Render.gpu_read_pixels`; this is a planning record.
pub fn scene_capture_frame(args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let viewport_id = args::rec_u64(args, "viewport_id").unwrap_or(0);
    Ok(args::record([
        ("viewport_id", Value::U64(viewport_id)),
        ("status", Value::String("capture_requested".into())),
        ("format", Value::String("rgba8".into())),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn scene_create_returns_empty_scene() {
        let mut m = BTreeMap::new();
        m.insert("name".into(), Value::String("test_scene".into()));
        let result = scene_create(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert_eq!(rec.get("name"), Some(&Value::String("test_scene".into())));
                assert!(matches!(rec.get("nodes"), Some(Value::List(_))));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn scene_add_node_returns_coordinates() {
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::U64(1));
        m.insert("x".into(), Value::F64(10.0));
        m.insert("y".into(), Value::F64(20.0));
        m.insert("z".into(), Value::F64(30.0));
        let result = scene_add_node(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn scene_set_transform_defaults() {
        let mut m = BTreeMap::new();
        m.insert("node_id".into(), Value::U64(1));
        let result = scene_set_transform(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert_eq!(rec.get("sx"), Some(&Value::F64(1.0)));
                assert_eq!(rec.get("sy"), Some(&Value::F64(1.0)));
                assert_eq!(rec.get("sz"), Some(&Value::F64(1.0)));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn scene_set_viewport_returns_config() {
        let mut m = BTreeMap::new();
        m.insert("width".into(), Value::U64(1920));
        m.insert("height".into(), Value::U64(1080));
        let result = scene_set_viewport(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert_eq!(rec.get("width"), Some(&Value::U64(1920)));
                assert_eq!(rec.get("format"), Some(&Value::String("rgba8unorm".into())));
            }
            _ => panic!("expected record"),
        }
    }
}
