//! Present a Vibe `Render.scene` through webizen-render.
//!
//! The engine authors the contract. This host draws it (PNG data URI for the
//! webview; native swapchain stays on `/gpu-viewport`).

use super::poet::PoetHarnessState;
use vibe::Value;
use qualia_core_db::poet_host::invoke::ids;
use serde::Serialize;
use tauri::State;
use webizen_render::scene_contract::{
    RenderScene, SceneCamera, SceneEdge, SceneFace, SceneNode, ScenePoint, Tensor10DProjection,
};

#[derive(Serialize)]
pub struct PoetRenderResult {
    pub ok: bool,
    pub kind: String,
    pub honesty: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub face_count: usize,
    pub data_uri: Option<String>,
    pub diagnostic: Option<String>,
    pub contract: &'static str,
}

#[tauri::command]
pub fn poet_render_preview(
    state: State<PoetHarnessState>,
    kind: String,
    width: Option<u32>,
    height: Option<u32>,
) -> PoetRenderResult {
    let mut snap = state.snap.lock().expect("poet snapshot");
    let args = {
        let mut m = std::collections::BTreeMap::new();
        m.insert("kind".into(), Value::String(kind.clone()));
        Value::Record(m)
    };
    let value = match snap.invoke_id(ids::RENDER_SCENE, args) {
        Ok(v) => v,
        Err(e) => {
            return PoetRenderResult {
                ok: false,
                kind,
                honesty: snap.honesty().into(),
                node_count: 0,
                edge_count: 0,
                face_count: 0,
                data_uri: None,
                diagnostic: Some(e.to_json()),
                contract: "webizen_render::scene_contract::RenderScene",
            };
        }
    };
    let scene = scene_from_value(&value);
    let w = width.unwrap_or(960).clamp(160, 1920);
    let h = height.unwrap_or(480).clamp(120, 1080);
    let uri = webizen_render::render_scene_data_uri(&scene, w, h);
    let ok = uri.is_some();
    PoetRenderResult {
        ok,
        kind: rec_str(&value, "kind").unwrap_or(kind),
        honesty: if ok { "live" } else { "partial" }.into(),
        node_count: scene.nodes.len(),
        edge_count: scene.edges.len(),
        face_count: scene.faces.len(),
        data_uri: uri,
        diagnostic: if ok {
            None
        } else {
            Some("webizen-render returned no frame (no GPU adapter or empty scene)".into())
        },
        contract: "webizen_render::scene_contract::RenderScene",
    }
}

fn scene_from_value(v: &Value) -> RenderScene {
    let mut scene = RenderScene::new();
    if let Some(bg) = rec_str(v, "background") {
        scene.set_background(bg);
    } else {
        scene.set_background("#07090e");
    }
    if let Value::Record(cam) = rec(v, "camera").unwrap_or(&Value::Null) {
        scene.set_camera(SceneCamera {
            position: [
                num(cam, "x").unwrap_or(0.0),
                num(cam, "y").unwrap_or(0.0),
                num(cam, "z").unwrap_or(2.4),
            ],
            target: [0.5, 0.5, 0.0],
            fov: num(cam, "fov").unwrap_or(58.0),
        });
    }
    if let Some(Value::List(nodes)) = rec(v, "nodes") {
        for node in nodes {
            if let Some(n) = node_from(node) {
                scene.add_node(n);
            }
        }
    }
    if let Some(Value::List(edges)) = rec(v, "edges") {
        for e in edges {
            if let Some(edge) = edge_from(e) {
                scene.add_edge(edge);
            }
        }
    }
    if let Some(Value::List(faces)) = rec(v, "faces") {
        for f in faces {
            if let Some(face) = face_from(f) {
                scene.add_face(face);
            }
        }
    }
    scene
}

fn node_from(v: &Value) -> Option<SceneNode> {
    let Value::Record(m) = v else {
        return None;
    };
    let sigma = num(m, "sigma").unwrap_or(0.4);
    Some(SceneNode {
        id: match m.get("id") {
            Some(Value::String(s)) => s.clone(),
            _ => "node".into(),
        },
        position: ScenePoint {
            x: num(m, "x")?,
            y: num(m, "y")?,
            z: num(m, "z").unwrap_or(0.0),
        },
        color: match m.get("color") {
            Some(Value::String(s)) => s.clone(),
            _ => "#00d2ff".into(),
        },
        radius: num(m, "radius").unwrap_or(6.0),
        alpha: num(m, "alpha").unwrap_or(0.92),
        pulse_rate: 0.4,
        tensor: Tensor10DProjection {
            sigma,
            x: num(m, "x").unwrap_or(0.5),
            y: num(m, "y").unwrap_or(0.5),
            z: num(m, "z").unwrap_or(0.0),
            alpha: num(m, "alpha").unwrap_or(0.92),
            ..Tensor10DProjection::default()
        },
        ..SceneNode::default()
    })
}

fn edge_from(v: &Value) -> Option<SceneEdge> {
    let Value::Record(m) = v else {
        return None;
    };
    Some(SceneEdge {
        from: point(m.get("from")?)?,
        to: point(m.get("to")?)?,
        color: match m.get("color") {
            Some(Value::String(s)) => s.clone(),
            _ => "#334155".into(),
        },
        width: num(m, "width").unwrap_or(1.2),
        alpha: num(m, "alpha").unwrap_or(0.8),
    })
}

fn face_from(v: &Value) -> Option<SceneFace> {
    let Value::Record(m) = v else {
        return None;
    };
    let Value::List(vs) = m.get("vertices")? else {
        return None;
    };
    let vertices: Vec<ScenePoint> = vs.iter().filter_map(point).collect();
    if vertices.len() < 3 {
        return None;
    }
    Some(SceneFace {
        vertices,
        color: match m.get("color") {
            Some(Value::String(s)) => s.clone(),
            _ => "#14532d".into(),
        },
        alpha: num(m, "alpha").unwrap_or(0.28),
    })
}

fn point(v: &Value) -> Option<ScenePoint> {
    match v {
        Value::List(xs) if xs.len() >= 2 => Some(ScenePoint {
            x: as_f64(&xs[0])?,
            y: as_f64(&xs[1])?,
            z: xs.get(2).and_then(as_f64).unwrap_or(0.0),
        }),
        Value::Record(m) => Some(ScenePoint {
            x: num(m, "x")?,
            y: num(m, "y")?,
            z: num(m, "z").unwrap_or(0.0),
        }),
        _ => None,
    }
}

fn rec<'a>(v: &'a Value, k: &str) -> Option<&'a Value> {
    match v {
        Value::Record(m) => m.get(k),
        _ => None,
    }
}

fn rec_str(v: &Value, k: &str) -> Option<String> {
    match rec(v, k) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn num(m: &std::collections::BTreeMap<String, Value>, k: &str) -> Option<f64> {
    m.get(k).and_then(as_f64)
}

fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::F64(n) => Some(*n),
        Value::I64(n) => Some(*n as f64),
        Value::U64(n) => Some(*n as f64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_record_is_empty_scene() {
        let scene = scene_from_value(&Value::Record(Default::default()));
        assert!(scene.nodes.is_empty());
    }
}
