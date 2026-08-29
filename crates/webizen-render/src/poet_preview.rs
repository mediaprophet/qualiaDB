//! Native `webizen-render` provider for the POET loopback service boundary.

use std::{collections::BTreeMap, sync::Arc};

use qualia_core_db::{
    poet_host::{invoke::ids, PoetSnapshot},
    services::poet_render_api::{
        register_poet_render_provider as register_provider, PoetRenderProvider, PoetRenderRequest,
        PoetRenderResponse,
    },
};
use vibe::Value;

use crate::scene_contract::{
    RenderScene, SceneCamera, SceneEdge, SceneFace, SceneNode, ScenePoint, Tensor10DProjection,
};

pub struct WebizenPoetRenderProvider;

/// Register the real native renderer before the loopback server starts.
pub fn register_poet_render_provider() -> Result<(), &'static str> {
    register_provider(Arc::new(WebizenPoetRenderProvider))
}

impl PoetRenderProvider for WebizenPoetRenderProvider {
    fn render_preview(&self, request: &PoetRenderRequest) -> PoetRenderResponse {
        let width = request.width.unwrap_or(960);
        let height = request.height.unwrap_or(480);
        let mut snapshot = PoetSnapshot::from_daemon();
        let mut args = BTreeMap::new();
        args.insert("kind".into(), Value::String(request.kind.clone()));
        let value = match snapshot.invoke_id(ids::RENDER_SCENE, Value::Record(args)) {
            Ok(value) => value,
            Err(error) => {
                return response(
                    request,
                    width,
                    height,
                    false,
                    0,
                    0,
                    0,
                    None,
                    Some(error.to_string()),
                );
            }
        };

        let scene = scene_from_value(&value);
        let data_uri = crate::render_scene_data_uri(&scene, width, height);
        response(
            request,
            width,
            height,
            data_uri.is_some(),
            scene.nodes.len(),
            scene.edges.len(),
            scene.faces.len(),
            data_uri,
            None,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn response(
    request: &PoetRenderRequest,
    width: u32,
    height: u32,
    ok: bool,
    node_count: usize,
    edge_count: usize,
    face_count: usize,
    data_uri: Option<String>,
    diagnostic: Option<String>,
) -> PoetRenderResponse {
    PoetRenderResponse {
        ok,
        kind: request.kind.clone(),
        honesty: if ok { "live" } else { "unavailable" }.into(),
        width,
        height,
        node_count,
        edge_count,
        face_count,
        data_uri,
        diagnostic: diagnostic.or_else(|| {
            (!ok).then(|| {
                "webizen-render returned no frame; no compatible GPU adapter may be available"
                    .into()
            })
        }),
        contract: "webizen_render::scene_contract::RenderScene".into(),
    }
}

pub fn scene_from_value(value: &Value) -> RenderScene {
    let mut scene = RenderScene::new();
    scene.set_background(rec_str(value, "background").unwrap_or_else(|| "#07090e".into()));
    if let Some(Value::Record(camera)) = rec(value, "camera") {
        scene.set_camera(SceneCamera {
            position: [
                num(camera, "x").unwrap_or(0.0),
                num(camera, "y").unwrap_or(0.0),
                num(camera, "z").unwrap_or(2.4),
            ],
            target: [0.5, 0.5, 0.0],
            fov: num(camera, "fov").unwrap_or(58.0),
        });
    }
    if let Some(Value::List(nodes)) = rec(value, "nodes") {
        for node in nodes.iter().filter_map(node_from) {
            scene.add_node(node);
        }
    }
    if let Some(Value::List(edges)) = rec(value, "edges") {
        for edge in edges.iter().filter_map(edge_from) {
            scene.add_edge(edge);
        }
    }
    if let Some(Value::List(faces)) = rec(value, "faces") {
        for face in faces.iter().filter_map(face_from) {
            scene.add_face(face);
        }
    }
    scene
}

fn node_from(value: &Value) -> Option<SceneNode> {
    let Value::Record(record) = value else {
        return None;
    };
    let sigma = num(record, "sigma").unwrap_or(0.4);
    Some(SceneNode {
        id: string(record, "id").unwrap_or_else(|| "node".into()),
        position: ScenePoint {
            x: num(record, "x")?,
            y: num(record, "y")?,
            z: num(record, "z").unwrap_or(0.0),
        },
        color: string(record, "color").unwrap_or_else(|| "#00d2ff".into()),
        radius: num(record, "radius").unwrap_or(6.0),
        alpha: num(record, "alpha").unwrap_or(0.92),
        pulse_rate: 0.4,
        tensor: Tensor10DProjection {
            sigma,
            x: num(record, "x").unwrap_or(0.5),
            y: num(record, "y").unwrap_or(0.5),
            z: num(record, "z").unwrap_or(0.0),
            alpha: num(record, "alpha").unwrap_or(0.92),
            ..Tensor10DProjection::default()
        },
        ..SceneNode::default()
    })
}

fn edge_from(value: &Value) -> Option<SceneEdge> {
    let Value::Record(record) = value else {
        return None;
    };
    Some(SceneEdge {
        from: point(record.get("from")?)?,
        to: point(record.get("to")?)?,
        color: string(record, "color").unwrap_or_else(|| "#334155".into()),
        width: num(record, "width").unwrap_or(1.2),
        alpha: num(record, "alpha").unwrap_or(0.8),
    })
}

fn face_from(value: &Value) -> Option<SceneFace> {
    let Value::Record(record) = value else {
        return None;
    };
    let Value::List(values) = record.get("vertices")? else {
        return None;
    };
    let vertices: Vec<ScenePoint> = values.iter().filter_map(point).collect();
    (vertices.len() >= 3).then(|| SceneFace {
        vertices,
        color: string(record, "color").unwrap_or_else(|| "#14532d".into()),
        alpha: num(record, "alpha").unwrap_or(0.28),
    })
}

fn point(value: &Value) -> Option<ScenePoint> {
    match value {
        Value::List(values) if values.len() >= 2 => Some(ScenePoint {
            x: as_f64(&values[0])?,
            y: as_f64(&values[1])?,
            z: values.get(2).and_then(as_f64).unwrap_or(0.0),
        }),
        Value::Record(record) => Some(ScenePoint {
            x: num(record, "x")?,
            y: num(record, "y")?,
            z: num(record, "z").unwrap_or(0.0),
        }),
        _ => None,
    }
}

fn rec<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Record(record) => record.get(key),
        _ => None,
    }
}

fn rec_str(value: &Value, key: &str) -> Option<String> {
    match rec(value, key) {
        Some(Value::String(value)) => Some(value.clone()),
        _ => None,
    }
}

fn string(record: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    match record.get(key) {
        Some(Value::String(value)) => Some(value.clone()),
        _ => None,
    }
}

fn num(record: &BTreeMap<String, Value>, key: &str) -> Option<f64> {
    record.get(key).and_then(as_f64)
}

fn as_f64(value: &Value) -> Option<f64> {
    match value {
        Value::F64(value) => Some(*value),
        Value::I64(value) => Some(*value as f64),
        Value::U64(value) => Some(*value as f64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_record_maps_to_empty_scene() {
        let scene = scene_from_value(&Value::Record(BTreeMap::new()));
        assert!(scene.nodes.is_empty());
        assert!(scene.edges.is_empty());
        assert!(scene.faces.is_empty());
    }

    #[test]
    fn unavailable_response_never_includes_a_synthetic_frame() {
        let request = PoetRenderRequest {
            kind: "map".into(),
            width: Some(320),
            height: Some(180),
        };
        let result = response(&request, 320, 180, false, 0, 0, 0, None, None);
        assert!(!result.ok);
        assert_eq!(result.honesty, "unavailable");
        assert!(result.data_uri.is_none());
        assert!(result.diagnostic.is_some());
    }
}
