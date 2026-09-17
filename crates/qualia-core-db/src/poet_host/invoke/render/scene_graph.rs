//! Scene graph invoke extensions — high-level scene operations wrapping
//! the existing render infrastructure.

use super::super::args;
use vibe::{Diagnostic, Span, Value};

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
pub fn scene_add_camera(args: &Value, _span: Span) -> Result<Value, Diagnostic> {
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
pub fn scene_set_clear_colour(args: &Value, _span: Span) -> Result<Value, Diagnostic> {
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

// ── N7: Scene graph build-new seams ──────────────────────────────────────────

/// `Scene.add_light` — add a light source to the scene.
///
/// Takes `light_type` ("point"/"directional"/"spot"/"ambient"),
/// `colour` (list of 3 f64), `intensity` (f64), and position/direction
/// depending on light type. For spot lights, `inner_cone` and `outer_cone`
/// (radians) are accepted.
pub fn scene_add_light(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::render::scene_graph::Light;

    let light_type_str = args::rec_str(args, "light_type")
        .ok_or_else(|| args::bad(span, "Scene.add_light needs light_type"))?;
    let colour_list = args::rec_f64_list(args, "colour").unwrap_or_else(|| vec![1.0; 3]);
    if colour_list.len() < 3 {
        return Err(args::bad(
            span,
            "Scene.add_light: colour must have 3 elements",
        ));
    }
    let colour = [
        colour_list[0] as f32,
        colour_list[1] as f32,
        colour_list[2] as f32,
    ];
    let intensity = args::rec_f64(args, "intensity").unwrap_or(1.0) as f32;

    let light = match light_type_str {
        "point" => {
            let pos = read_vec3(args, "position", [0.0; 3]);
            Light::point(pos, colour, intensity)
        }
        "directional" => {
            let dir = read_vec3(args, "direction", [0.0, -1.0, 0.0]);
            Light::directional(dir, colour, intensity)
        }
        "spot" => {
            let pos = read_vec3(args, "position", [0.0; 3]);
            let dir = read_vec3(args, "direction", [0.0, -1.0, 0.0]);
            let inner = args::rec_f64(args, "inner_cone").unwrap_or(0.5) as f32;
            let outer = args::rec_f64(args, "outer_cone").unwrap_or(0.8) as f32;
            Light::spot(pos, dir, colour, intensity, inner, outer)
        }
        "ambient" => Light::ambient(colour, intensity),
        _ => {
            return Err(args::bad(
                span,
                format!("Scene.add_light: unknown light_type '{light_type_str}'"),
            ))
        }
    };

    Ok(args::record([
        ("light_type", Value::String(light_type_str.to_string())),
        ("intensity", Value::F64(intensity as f64)),
        (
            "colour",
            Value::List(colour.iter().map(|&c| Value::F64(c as f64)).collect()),
        ),
        ("cast_shadows", Value::Bool(light.cast_shadows)),
        ("range", Value::F64(light.range as f64)),
        ("status", Value::String("light_added".into())),
    ]))
}

/// `Scene.link_semantic` — link a scene node to a semantic entity IRI.
///
/// Takes `node_id` (string), `semantic_iri` (string), `link_type` (string),
/// and optional `confidence` (f64).
pub fn scene_link_semantic(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let node_id = args::rec_str(args, "node_id")
        .ok_or_else(|| args::bad(span, "Scene.link_semantic needs node_id"))?;
    let semantic_iri = args::rec_str(args, "semantic_iri")
        .ok_or_else(|| args::bad(span, "Scene.link_semantic needs semantic_iri"))?;
    let link_type = args::rec_str(args, "link_type").unwrap_or("represents");
    let confidence = args::rec_f64(args, "confidence");

    let mut rec = std::collections::BTreeMap::new();
    rec.insert("node_id".into(), Value::String(node_id.to_string()));
    rec.insert(
        "semantic_iri".into(),
        Value::String(semantic_iri.to_string()),
    );
    rec.insert("link_type".into(), Value::String(link_type.to_string()));
    if let Some(c) = confidence {
        rec.insert("confidence".into(), Value::F64(c));
    }
    rec.insert("status".into(), Value::String("linked".into()));
    Ok(Value::Record(rec))
}

/// `Scene.duplicate_node` — duplicate a scene node with a new ID.
///
/// Takes `source_id` (string), `new_id` (string), and optional `parent` (string).
pub fn scene_duplicate_node(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let source_id = args::rec_str(args, "source_id")
        .ok_or_else(|| args::bad(span, "Scene.duplicate_node needs source_id"))?;
    let new_id = args::rec_str(args, "new_id")
        .ok_or_else(|| args::bad(span, "Scene.duplicate_node needs new_id"))?;
    let parent = args::rec_str(args, "parent");

    let mut rec = std::collections::BTreeMap::new();
    rec.insert("source_id".into(), Value::String(source_id.to_string()));
    rec.insert("new_id".into(), Value::String(new_id.to_string()));
    if let Some(p) = parent {
        rec.insert("parent".into(), Value::String(p.to_string()));
    }
    rec.insert("status".into(), Value::String("duplicated".into()));
    Ok(Value::Record(rec))
}

/// `Scene.set_render_budget` — set the per-frame render budget.
///
/// Takes `budget_ms` (f64).
pub fn scene_set_render_budget(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let budget_ms = args::rec_f64(args, "budget_ms")
        .ok_or_else(|| args::bad(span, "Scene.set_render_budget needs budget_ms"))?;

    Ok(args::record([
        ("budget_ms", Value::F64(budget_ms)),
        ("status", Value::String("budget_set".into())),
    ]))
}

/// `Scene.ik_look_at` — rotate a kinematic chain so the end effector points
/// toward a target.
///
/// Takes `joints` (list of [x, y, z] lists) and `target` ([x, y, z] list).
/// Returns updated joint positions, convergence status, and final distance.
pub fn scene_ik_look_at(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::render::scene_graph::look_at_ik;

    let joints = read_joint_list(args, "joints")
        .ok_or_else(|| args::bad(span, "Scene.ik_look_at needs joints"))?;
    let target = read_vec3(args, "target", [0.0; 3]);

    let result = look_at_ik(&joints, target);
    let joint_values: Vec<Value> = result
        .joint_positions
        .iter()
        .map(|p| {
            Value::List(vec![
                Value::F64(p[0] as f64),
                Value::F64(p[1] as f64),
                Value::F64(p[2] as f64),
            ])
        })
        .collect();

    Ok(args::record([
        ("joints", Value::List(joint_values)),
        ("converged", Value::Bool(result.converged)),
        ("iterations", Value::U64(result.iterations as u64)),
        ("final_distance", Value::F64(result.final_distance as f64)),
    ]))
}

/// `Scene.ik_ccd` — CCD inverse kinematics solver.
///
/// Takes `joints` (list of [x, y, z]), `target` ([x, y, z]),
/// `max_iterations` (u64), and `tolerance` (f64).
pub fn scene_ik_ccd(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::render::scene_graph::ccd_ik;

    let joints = read_joint_list(args, "joints")
        .ok_or_else(|| args::bad(span, "Scene.ik_ccd needs joints"))?;
    let target = read_vec3(args, "target", [0.0; 3]);
    let max_iter = args::rec_u64(args, "max_iterations").unwrap_or(50) as u32;
    let tolerance = args::rec_f64(args, "tolerance").unwrap_or(0.01) as f32;

    let result = ccd_ik(&joints, target, max_iter, tolerance);
    let joint_values: Vec<Value> = result
        .joint_positions
        .iter()
        .map(|p| {
            Value::List(vec![
                Value::F64(p[0] as f64),
                Value::F64(p[1] as f64),
                Value::F64(p[2] as f64),
            ])
        })
        .collect();

    Ok(args::record([
        ("joints", Value::List(joint_values)),
        ("converged", Value::Bool(result.converged)),
        ("iterations", Value::U64(result.iterations as u64)),
        ("final_distance", Value::F64(result.final_distance as f64)),
    ]))
}

/// `Scene.smooth_damp` — smoothly damp a scalar toward a target.
///
/// Takes `current` (f64), `target` (f64), `velocity` (f64),
/// `smooth_time` (f64), `max_speed` (f64), and `delta_time` (f64).
pub fn scene_smooth_damp(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::render::scene_graph::smooth_damp;

    let current = args::rec_f64(args, "current")
        .ok_or_else(|| args::bad(span, "Scene.smooth_damp needs current"))?
        as f32;
    let target = args::rec_f64(args, "target")
        .ok_or_else(|| args::bad(span, "Scene.smooth_damp needs target"))? as f32;
    let mut velocity = args::rec_f64(args, "velocity").unwrap_or(0.0) as f32;
    let smooth_time = args::rec_f64(args, "smooth_time").unwrap_or(0.3) as f32;
    let max_speed = args::rec_f64(args, "max_speed").unwrap_or(f64::INFINITY) as f32;
    let delta_time = args::rec_f64(args, "delta_time").unwrap_or(0.016) as f32;

    let result = smooth_damp(
        current,
        target,
        &mut velocity,
        smooth_time,
        max_speed,
        delta_time,
    );

    Ok(args::record([
        ("value", Value::F64(result as f64)),
        ("velocity", Value::F64(velocity as f64)),
    ]))
}

/// `Scene.smooth_damp_vec3` — smoothly damp a 3D vector toward a target.
///
/// Takes `current` ([x,y,z]), `target` ([x,y,z]), `velocity` ([x,y,z]),
/// `smooth_time`, `max_speed`, and `delta_time`.
pub fn scene_smooth_damp_vec3(args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    use crate::render::scene_graph::smooth_damp_vec3;

    let current = read_vec3(args, "current", [0.0; 3]);
    let target = read_vec3(args, "target", [0.0; 3]);
    let mut velocity = read_vec3(args, "velocity", [0.0; 3]);
    let smooth_time = args::rec_f64(args, "smooth_time").unwrap_or(0.3) as f32;
    let max_speed = args::rec_f64(args, "max_speed").unwrap_or(f64::INFINITY) as f32;
    let delta_time = args::rec_f64(args, "delta_time").unwrap_or(0.016) as f32;

    let result = smooth_damp_vec3(
        current,
        target,
        &mut velocity,
        smooth_time,
        max_speed,
        delta_time,
    );

    Ok(args::record([
        (
            "value",
            Value::List(vec![
                Value::F64(result[0] as f64),
                Value::F64(result[1] as f64),
                Value::F64(result[2] as f64),
            ]),
        ),
        (
            "velocity",
            Value::List(vec![
                Value::F64(velocity[0] as f64),
                Value::F64(velocity[1] as f64),
                Value::F64(velocity[2] as f64),
            ]),
        ),
    ]))
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn read_vec3(args: &Value, key: &str, default: [f32; 3]) -> [f32; 3] {
    if let Some(list) = args::rec_f64_list(args, key) {
        if list.len() >= 3 {
            return [list[0] as f32, list[1] as f32, list[2] as f32];
        }
    }
    default
}

fn read_joint_list(args: &Value, key: &str) -> Option<Vec<[f32; 3]>> {
    let val = args::rec(args, key)?;
    let list = args::list(val)?;
    let mut joints = Vec::with_capacity(list.len());
    for entry in list {
        if let Value::List(coords) = entry {
            if coords.len() >= 3 {
                let x = args::as_f64(&coords[0])? as f32;
                let y = args::as_f64(&coords[1])? as f32;
                let z = args::as_f64(&coords[2])? as f32;
                joints.push([x, y, z]);
            }
        }
    }
    if joints.is_empty() {
        None
    } else {
        Some(joints)
    }
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

    #[test]
    fn scene_add_light_point() {
        let mut m = BTreeMap::new();
        m.insert("light_type".into(), Value::String("point".into()));
        m.insert("colour".into(), Value::List(vec![Value::F64(1.0); 3]));
        m.insert("intensity".into(), Value::F64(100.0));
        m.insert(
            "position".into(),
            Value::List(vec![Value::F64(1.0), Value::F64(2.0), Value::F64(3.0)]),
        );
        let result = scene_add_light(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn scene_add_light_directional() {
        let mut m = BTreeMap::new();
        m.insert("light_type".into(), Value::String("directional".into()));
        m.insert("intensity".into(), Value::F64(1.0));
        let result = scene_add_light(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn scene_add_light_unknown() {
        let mut m = BTreeMap::new();
        m.insert("light_type".into(), Value::String("unknown".into()));
        let result = scene_add_light(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_err());
    }

    #[test]
    fn scene_link_semantic_basic() {
        let mut m = BTreeMap::new();
        m.insert("node_id".into(), Value::String("node1".into()));
        m.insert(
            "semantic_iri".into(),
            Value::String("did:qualia:entity1".into()),
        );
        m.insert("link_type".into(), Value::String("represents".into()));
        m.insert("confidence".into(), Value::F64(0.9));
        let result = scene_link_semantic(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn scene_duplicate_node_basic() {
        let mut m = BTreeMap::new();
        m.insert("source_id".into(), Value::String("original".into()));
        m.insert("new_id".into(), Value::String("copy".into()));
        let result = scene_duplicate_node(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn scene_set_render_budget_basic() {
        let mut m = BTreeMap::new();
        m.insert("budget_ms".into(), Value::F64(16.6));
        let result = scene_set_render_budget(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn scene_ik_look_at_basic() {
        let mut m = BTreeMap::new();
        m.insert(
            "joints".into(),
            Value::List(vec![
                Value::List(vec![Value::F64(0.0); 3]),
                Value::List(vec![Value::F64(1.0), Value::F64(0.0), Value::F64(0.0)]),
            ]),
        );
        m.insert(
            "target".into(),
            Value::List(vec![Value::F64(0.0), Value::F64(1.0), Value::F64(0.0)]),
        );
        let result = scene_ik_look_at(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn scene_ik_ccd_basic() {
        let mut m = BTreeMap::new();
        m.insert(
            "joints".into(),
            Value::List(vec![
                Value::List(vec![Value::F64(0.0); 3]),
                Value::List(vec![Value::F64(1.0), Value::F64(0.0), Value::F64(0.0)]),
                Value::List(vec![Value::F64(2.0), Value::F64(0.0), Value::F64(0.0)]),
            ]),
        );
        m.insert(
            "target".into(),
            Value::List(vec![Value::F64(0.0), Value::F64(2.0), Value::F64(0.0)]),
        );
        m.insert("max_iterations".into(), Value::U64(50));
        m.insert("tolerance".into(), Value::F64(0.01));
        let result = scene_ik_ccd(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn scene_smooth_damp_basic() {
        let mut m = BTreeMap::new();
        m.insert("current".into(), Value::F64(0.0));
        m.insert("target".into(), Value::F64(10.0));
        m.insert("velocity".into(), Value::F64(0.0));
        m.insert("smooth_time".into(), Value::F64(0.3));
        m.insert("delta_time".into(), Value::F64(0.016));
        let result = scene_smooth_damp(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn scene_smooth_damp_vec3_basic() {
        let mut m = BTreeMap::new();
        m.insert("current".into(), Value::List(vec![Value::F64(0.0); 3]));
        m.insert(
            "target".into(),
            Value::List(vec![Value::F64(10.0), Value::F64(0.0), Value::F64(5.0)]),
        );
        m.insert("velocity".into(), Value::List(vec![Value::F64(0.0); 3]));
        let result = scene_smooth_damp_vec3(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }
}
