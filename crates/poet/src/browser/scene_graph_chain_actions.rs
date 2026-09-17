//! Dual-path Tool Chest actions for remaining `Scene.*` ALL_BOUND ids (wave 22).
//!
//! Graph/build ops: create, node, transform, mesh, camera, render, viewport,
//! capture, light, semantic link, duplicate. Camera lerp / damp / IK / budget /
//! clear-colour live in `scene_chain_actions`. No Host widen — scopes must
//! already exist in `poet_host/invoke/ids.rs`.

use serde_json::{json, Value};
use web_sys::{Document, Element};

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn selected_source(document: &Document) -> Option<String> {
    let container = selected_container(document)?;
    let text = container
        .query_selector(".vibe-editor, .vibe-editor-textarea, .doc-editor, .sheet-grid")
        .ok()
        .flatten()
        .and_then(|editor| editor.text_content())
        .or_else(|| container.text_content())?;
    let bounded: String = text.chars().take(16_384).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

fn parse_numbers(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|' | '\n' | '\r'))
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(64)
        .collect()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn string_attr(el: Option<&Element>, name: &str) -> Option<String> {
    el.and_then(|e| e.get_attribute(name))
        .filter(|s| !s.trim().is_empty())
}

fn first_token(source: &str) -> Option<String> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|'))
        .map(str::trim)
        .find(|t| !t.is_empty() && t.parse::<f64>().is_err())
        .map(|t| t.chars().take(64).collect())
}

fn nth_token(source: &str, n: usize) -> Option<String> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|'))
        .map(str::trim)
        .filter(|t| !t.is_empty() && t.parse::<f64>().is_err())
        .nth(n)
        .map(|t| t.chars().take(64).collect())
}

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: Value,
) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(cap_id, &local_message);
        super::interactions::show_tool_status(
            document,
            &label,
            &report.message,
            report.status_kind,
        );
        return;
    }
    super::interactions::show_tool_status(
        document,
        &label,
        &format!("Running {cap_id}…"),
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::native_daemon::daemon_invoke(cap_id, args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok(cap_id, &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    cap_id,
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("capability invoke failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(cap_id, &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}

fn as_u64(n: f64) -> u64 {
    n.round().clamp(0.0, u64::MAX as f64) as u64
}

fn resolve_scene_name(container: Option<&Element>, source: &str) -> String {
    string_attr(container, "data-scene")
        .or_else(|| string_attr(container, "data-name"))
        .or_else(|| first_token(source))
        .unwrap_or_else(|| "scene-1".into())
}

fn resolve_node_u64(container: Option<&Element>, nums: &[f64]) -> u64 {
    numeric_attr(container, "data-node-id")
        .or_else(|| numeric_attr(container, "data-id"))
        .or_else(|| nums.first().copied())
        .map(as_u64)
        .unwrap_or(1)
}

fn id_consumed_from_nums(container: Option<&Element>, nums: &[f64]) -> bool {
    numeric_attr(container, "data-node-id").is_none()
        && numeric_attr(container, "data-id").is_none()
        && !nums.is_empty()
}

fn resolve_xyz(
    container: Option<&Element>,
    nums: &[f64],
    offset: usize,
    fallback: [f64; 3],
) -> [f64; 3] {
    [
        numeric_attr(container, "data-x")
            .or_else(|| nums.get(offset).copied())
            .unwrap_or(fallback[0]),
        numeric_attr(container, "data-y")
            .or_else(|| nums.get(offset + 1).copied())
            .unwrap_or(fallback[1]),
        numeric_attr(container, "data-z")
            .or_else(|| nums.get(offset + 2).copied())
            .unwrap_or(fallback[2]),
    ]
}

fn resolve_transform(
    container: Option<&Element>,
    nums: &[f64],
) -> (u64, [f64; 3], [f64; 3], [f64; 3]) {
    let node_id = resolve_node_u64(container, nums);
    let off = if id_consumed_from_nums(container, nums) {
        1
    } else {
        0
    };
    let t = [
        numeric_attr(container, "data-tx")
            .or_else(|| nums.get(off).copied())
            .unwrap_or(0.0),
        numeric_attr(container, "data-ty")
            .or_else(|| nums.get(off + 1).copied())
            .unwrap_or(0.0),
        numeric_attr(container, "data-tz")
            .or_else(|| nums.get(off + 2).copied())
            .unwrap_or(0.0),
    ];
    let r = [
        numeric_attr(container, "data-rx")
            .or_else(|| nums.get(off + 3).copied())
            .unwrap_or(0.0),
        numeric_attr(container, "data-ry")
            .or_else(|| nums.get(off + 4).copied())
            .unwrap_or(0.0),
        numeric_attr(container, "data-rz")
            .or_else(|| nums.get(off + 5).copied())
            .unwrap_or(0.0),
    ];
    let s = [
        numeric_attr(container, "data-sx")
            .or_else(|| nums.get(off + 6).copied())
            .unwrap_or(1.0),
        numeric_attr(container, "data-sy")
            .or_else(|| nums.get(off + 7).copied())
            .unwrap_or(1.0),
        numeric_attr(container, "data-sz")
            .or_else(|| nums.get(off + 8).copied())
            .unwrap_or(1.0),
    ];
    (node_id, t, r, s)
}

fn resolve_mesh_iri(container: Option<&Element>, source: &str) -> String {
    string_attr(container, "data-mesh-iri")
        .or_else(|| first_token(source))
        .unwrap_or_else(|| "mesh:default".into())
}

fn resolve_camera(container: Option<&Element>, nums: &[f64]) -> (f64, f64, f64, f64) {
    let x = numeric_attr(container, "data-x")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.0);
    let y = numeric_attr(container, "data-y")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(0.0);
    let z = numeric_attr(container, "data-z")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(5.0);
    let fov = numeric_attr(container, "data-fov")
        .or_else(|| nums.get(3).copied())
        .unwrap_or(60.0);
    (x, y, z, fov)
}

fn resolve_viewport(container: Option<&Element>, source: &str, nums: &[f64]) -> (u64, u64, String) {
    let width = numeric_attr(container, "data-width")
        .or_else(|| nums.first().copied())
        .map(as_u64)
        .unwrap_or(1920);
    let height = numeric_attr(container, "data-height")
        .or_else(|| nums.get(1).copied())
        .map(as_u64)
        .unwrap_or(1080);
    let format = string_attr(container, "data-format")
        .or_else(|| first_token(source))
        .unwrap_or_else(|| "rgba8unorm".into());
    (width, height, format)
}

fn resolve_light_type(container: Option<&Element>, source: &str) -> String {
    string_attr(container, "data-light-type")
        .or_else(|| {
            source
                .split_whitespace()
                .find(|t| {
                    matches!(
                        t.to_ascii_lowercase().as_str(),
                        "point" | "directional" | "spot" | "ambient"
                    )
                })
                .map(|s| s.to_ascii_lowercase())
        })
        .unwrap_or_else(|| "point".into())
}

fn resolve_colour(container: Option<&Element>, nums: &[f64]) -> [f64; 3] {
    [
        numeric_attr(container, "data-r")
            .or_else(|| nums.first().copied())
            .unwrap_or(1.0),
        numeric_attr(container, "data-g")
            .or_else(|| nums.get(1).copied())
            .unwrap_or(1.0),
        numeric_attr(container, "data-b")
            .or_else(|| nums.get(2).copied())
            .unwrap_or(1.0),
    ]
}

fn resolve_node_id_str(container: Option<&Element>, source: &str, nums: &[f64]) -> String {
    string_attr(container, "data-node-id")
        .or_else(|| first_token(source))
        .or_else(|| nums.first().map(|n| as_u64(*n).to_string()))
        .unwrap_or_else(|| "1".into())
}

fn resolve_semantic_iri(container: Option<&Element>, source: &str) -> String {
    string_attr(container, "data-semantic-iri")
        .or_else(|| {
            source
                .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|'))
                .map(str::trim)
                .find(|t| t.contains(':') && t.parse::<f64>().is_err())
                .map(|t| t.chars().take(128).collect())
        })
        .unwrap_or_else(|| "q42:entity".into())
}

fn resolve_duplicate(
    container: Option<&Element>,
    source: &str,
    nums: &[f64],
) -> (String, String, Option<String>) {
    let source_id = string_attr(container, "data-source-id")
        .or_else(|| nth_token(source, 0))
        .or_else(|| nums.first().map(|n| as_u64(*n).to_string()))
        .unwrap_or_else(|| "1".into());
    let new_id = string_attr(container, "data-new-id")
        .or_else(|| nth_token(source, 1))
        .or_else(|| nums.get(1).map(|n| as_u64(*n).to_string()))
        .unwrap_or_else(|| "2".into());
    let parent = string_attr(container, "data-parent").or_else(|| nth_token(source, 2));
    (source_id, new_id, parent)
}

/// `Scene.create` — `{ name }`.
pub(super) fn run_create(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let name = resolve_scene_name(container.as_ref(), &source);
    invoke_dual(
        document,
        label,
        "Scene.create",
        format!("Scene create sketch name={name}"),
        json!({ "name": name }),
    );
}

/// `Scene.add_node` — `{ id, x?, y?, z? }`.
pub(super) fn run_add_node(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let id = resolve_node_u64(container.as_ref(), &nums);
    let off = if id_consumed_from_nums(container.as_ref(), &nums) {
        1
    } else {
        0
    };
    let xyz = resolve_xyz(container.as_ref(), &nums, off, [0.0, 0.0, 0.0]);
    invoke_dual(
        document,
        label,
        "Scene.add_node",
        format!(
            "Scene add_node sketch id={id} xyz=[{:.3},{:.3},{:.3}]",
            xyz[0], xyz[1], xyz[2]
        ),
        json!({ "id": id, "x": xyz[0], "y": xyz[1], "z": xyz[2] }),
    );
}

/// `Scene.set_transform` — `{ node_id, tx?, ty?, tz?, rx?, ry?, rz?, sx?, sy?, sz? }`.
pub(super) fn run_set_transform(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (node_id, t, r, s) = resolve_transform(container.as_ref(), &nums);
    invoke_dual(
        document,
        label,
        "Scene.set_transform",
        format!("Scene set_transform sketch node_id={node_id}"),
        json!({
            "node_id": node_id,
            "tx": t[0], "ty": t[1], "tz": t[2],
            "rx": r[0], "ry": r[1], "rz": r[2],
            "sx": s[0], "sy": s[1], "sz": s[2],
        }),
    );
}

/// `Scene.set_mesh` — `{ node_id, mesh_iri }`.
pub(super) fn run_set_mesh(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let node_id = resolve_node_u64(container.as_ref(), &nums);
    let mesh_iri = resolve_mesh_iri(container.as_ref(), &source);
    invoke_dual(
        document,
        label,
        "Scene.set_mesh",
        format!("Scene set_mesh sketch node_id={node_id} mesh_iri={mesh_iri}"),
        json!({ "node_id": node_id, "mesh_iri": mesh_iri }),
    );
}

/// `Scene.add_camera` — `{ x?, y?, z?, fov? }`.
pub(super) fn run_add_camera(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y, z, fov) = resolve_camera(container.as_ref(), &nums);
    invoke_dual(
        document,
        label,
        "Scene.add_camera",
        format!("Scene add_camera sketch xyz=[{x:.3},{y:.3},{z:.3}] fov={fov:.1}"),
        json!({ "x": x, "y": y, "z": z, "fov": fov }),
    );
}

/// `Scene.render` — `{ scene, camera_id? }`.
pub(super) fn run_render(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let scene = resolve_scene_name(container.as_ref(), &source);
    let camera_id = numeric_attr(container.as_ref(), "data-camera-id")
        .or_else(|| nums.first().copied())
        .map(as_u64)
        .unwrap_or(0);
    invoke_dual(
        document,
        label,
        "Scene.render",
        format!("Scene render sketch scene={scene} camera_id={camera_id}"),
        json!({ "scene": scene, "camera_id": camera_id }),
    );
}

/// `Scene.set_viewport` — `{ width, height, format? }`.
pub(super) fn run_set_viewport(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (width, height, format) = resolve_viewport(container.as_ref(), &source, &nums);
    invoke_dual(
        document,
        label,
        "Scene.set_viewport",
        format!("Scene set_viewport sketch {width}x{height} format={format}"),
        json!({ "width": width, "height": height, "format": format }),
    );
}

/// `Scene.capture_frame` — `{ viewport_id? }`.
pub(super) fn run_capture_frame(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let viewport_id = numeric_attr(container.as_ref(), "data-viewport-id")
        .or_else(|| nums.first().copied())
        .map(as_u64)
        .unwrap_or(0);
    invoke_dual(
        document,
        label,
        "Scene.capture_frame",
        format!("Scene capture_frame sketch viewport_id={viewport_id}"),
        json!({ "viewport_id": viewport_id }),
    );
}

/// `Scene.add_light` — `{ light_type, colour?, intensity?, position?, direction? }`.
pub(super) fn run_add_light(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let light_type = resolve_light_type(container.as_ref(), &source);
    let colour = resolve_colour(container.as_ref(), &nums);
    let colour_from_nums = nums.len() >= 3;
    let intensity = numeric_attr(container.as_ref(), "data-intensity")
        .or_else(|| nums.get(if colour_from_nums { 3 } else { 0 }).copied())
        .unwrap_or(1.0);
    let pos_off = if colour_from_nums { 4 } else { 1 };
    let position = resolve_xyz(container.as_ref(), &nums, pos_off, [0.0, 0.0, 0.0]);
    let direction = [
        nums.get(pos_off + 3).copied().unwrap_or(0.0),
        nums.get(pos_off + 4).copied().unwrap_or(-1.0),
        nums.get(pos_off + 5).copied().unwrap_or(0.0),
    ];
    let mut args = json!({
        "light_type": light_type,
        "colour": colour,
        "intensity": intensity,
        "position": position,
        "direction": direction,
    });
    if light_type == "spot" {
        let inner = numeric_attr(container.as_ref(), "data-inner-cone")
            .or_else(|| nums.get(pos_off + 6).copied())
            .unwrap_or(0.5);
        let outer = numeric_attr(container.as_ref(), "data-outer-cone")
            .or_else(|| nums.get(pos_off + 7).copied())
            .unwrap_or(0.8);
        args["inner_cone"] = json!(inner);
        args["outer_cone"] = json!(outer);
    }
    invoke_dual(
        document,
        label,
        "Scene.add_light",
        format!("Scene add_light sketch type={light_type} intensity={intensity:.3}"),
        args,
    );
}

/// `Scene.link_semantic` — `{ node_id, semantic_iri, link_type?, confidence? }` (node_id is a string).
pub(super) fn run_link_semantic(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let node_id = resolve_node_id_str(container.as_ref(), &source, &nums);
    let semantic_iri = resolve_semantic_iri(container.as_ref(), &source);
    let link_type = string_attr(container.as_ref(), "data-link-type")
        .unwrap_or_else(|| "represents".into());
    let mut args = json!({
        "node_id": node_id,
        "semantic_iri": semantic_iri,
        "link_type": link_type,
    });
    if let Some(confidence) = numeric_attr(container.as_ref(), "data-confidence")
        .or_else(|| nums.iter().copied().find(|n| (0.0..=1.0).contains(n)))
    {
        args["confidence"] = json!(confidence);
    }
    invoke_dual(
        document,
        label,
        "Scene.link_semantic",
        format!("Scene link_semantic sketch node_id={node_id} iri={semantic_iri}"),
        args,
    );
}

/// `Scene.duplicate_node` — `{ source_id, new_id, parent? }` (strings).
pub(super) fn run_duplicate_node(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (source_id, new_id, parent) = resolve_duplicate(container.as_ref(), &source, &nums);
    let mut args = json!({ "source_id": source_id, "new_id": new_id });
    if let Some(parent) = parent {
        args["parent"] = json!(parent);
    }
    invoke_dual(
        document,
        label,
        "Scene.duplicate_node",
        format!("Scene duplicate_node sketch {source_id}→{new_id}"),
        args,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_scene_wave22_graph_defaults() {
        assert_eq!(resolve_scene_name(None, ""), "scene-1");
        assert_eq!(resolve_scene_name(None, "stage-a 3"), "stage-a");
        assert_eq!(resolve_node_u64(None, &[]), 1);
        assert_eq!(resolve_node_u64(None, &[7.0, 1.0, 2.0, 3.0]), 7);
        let xyz = resolve_xyz(None, &[7.0, 1.0, 2.0, 3.0], 1, [0.0, 0.0, 0.0]);
        assert_eq!(xyz, [1.0, 2.0, 3.0]);
        let (node_id, t, r, s) = resolve_transform(None, &[]);
        assert_eq!(node_id, 1);
        assert_eq!(t, [0.0, 0.0, 0.0]);
        assert_eq!(r, [0.0, 0.0, 0.0]);
        assert_eq!(s, [1.0, 1.0, 1.0]);
        assert_eq!(resolve_mesh_iri(None, ""), "mesh:default");
        assert_eq!(resolve_mesh_iri(None, "mesh:box"), "mesh:box");
        let (x, y, z, fov) = resolve_camera(None, &[]);
        assert_eq!((x, y, z, fov), (0.0, 0.0, 5.0, 60.0));
        let (w, h, fmt) = resolve_viewport(None, "", &[]);
        assert_eq!((w, h, fmt.as_str()), (1920, 1080, "rgba8unorm"));
        assert_eq!(resolve_light_type(None, ""), "point");
        assert_eq!(resolve_light_type(None, "spot 1"), "spot");
        assert_eq!(resolve_colour(None, &[]), [1.0, 1.0, 1.0]);
        assert_eq!(resolve_node_id_str(None, "", &[]), "1");
        assert_eq!(resolve_semantic_iri(None, ""), "q42:entity");
        assert_eq!(
            resolve_semantic_iri(None, "did:q42:thing"),
            "did:q42:thing"
        );
        let (src, dst, parent) = resolve_duplicate(None, "", &[]);
        assert_eq!(src, "1");
        assert_eq!(dst, "2");
        assert!(parent.is_none());
        let (src2, dst2, _) = resolve_duplicate(None, "alpha beta", &[]);
        assert_eq!((src2, dst2), ("alpha".into(), "beta".into()));
        let nums = parse_numbers("1 2 3 scene-x");
        assert_eq!(nums, vec![1.0, 2.0, 3.0]);
    }
}
