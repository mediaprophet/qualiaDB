//! Dual-path Tool Chest actions for already-bound `Render.gpu_*` ids (wave 34).
//!
//! Honest sketches: these need a GPU/daemon. No new GPU Host binds.

use serde_json::json;
use web_sys::{Document, Element};

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: serde_json::Value,
) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(cap_id, &local_message);
        super::interactions::show_tool_status(document, &label, &report.message, report.status_kind);
        return;
    }
    super::interactions::show_tool_status(document, &label, &format!("Running {cap_id}…"), "running");
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

fn handle(document: &Document) -> u64 {
    numeric_attr(selected_container(document).as_ref(), "data-gpu-handle")
        .and_then(|v| (v >= 0.0).then_some(v as u64))
        .unwrap_or(0)
}

const TINY_WGSL: &str = "@compute @workgroup_size(1) fn main() {}";

pub(super) fn run_gpu_init(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Render.gpu_init",
        "gpu_init sketch 800×600 (needs GPU)".to_string(),
        json!({ "width": 800, "height": 600 }),
    );
}

pub(super) fn run_gpu_init_surface(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Render.gpu_init_surface",
        "gpu_init_surface sketch needs native hwnd".to_string(),
        json!({ "hwnd": 1, "width": 800, "height": 600 }),
    );
}

pub(super) fn run_gpu_render_frame(document: &Document, label: &str) {
    let handle = handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_render_frame",
        format!("gpu_render_frame sketch handle={handle} (needs GPU)"),
        json!({ "handle": handle, "time": 0.0 }),
    );
}

pub(super) fn run_gpu_read_pixels(document: &Document, label: &str) {
    let handle = handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_read_pixels",
        format!("gpu_read_pixels sketch handle={handle} (needs GPU)"),
        json!({ "handle": handle }),
    );
}

pub(super) fn run_gpu_upload_mesh(document: &Document, label: &str) {
    let handle = handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_upload_mesh",
        format!("gpu_upload_mesh sketch unit triangle handle={handle}"),
        json!({
            "handle": handle,
            "vertices": [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            "indices": [0, 1, 2],
        }),
    );
}

pub(super) fn run_gpu_upload_tensor(document: &Document, label: &str) {
    let handle = handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_upload_tensor",
        format!("gpu_upload_tensor sketch handle={handle} (needs GPU)"),
        json!({ "handle": handle, "bytes": [0, 0, 0, 0] }),
    );
}

pub(super) fn run_gpu_pick(document: &Document, label: &str) {
    let handle = handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_pick",
        format!("gpu_pick sketch handle={handle} (needs GPU)"),
        json!({ "handle": handle, "x": 0, "y": 0 }),
    );
}

pub(super) fn run_gpu_poll_pick(document: &Document, label: &str) {
    let handle = handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_poll_pick",
        format!("gpu_poll_pick sketch handle={handle}"),
        json!({ "handle": handle }),
    );
}

pub(super) fn run_gpu_resize(document: &Document, label: &str) {
    let handle = handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_resize",
        format!("gpu_resize sketch handle={handle}"),
        json!({ "handle": handle, "width": 800, "height": 600 }),
    );
}

pub(super) fn run_gpu_set_ambient(document: &Document, label: &str) {
    let handle = handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_set_ambient",
        format!("gpu_set_ambient sketch handle={handle}"),
        json!({ "handle": handle, "enabled": true }),
    );
}

pub(super) fn run_gpu_destroy(document: &Document, label: &str) {
    let handle = handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_destroy",
        format!("gpu_destroy sketch handle={handle}"),
        json!({ "handle": handle }),
    );
}

pub(super) fn run_gpu_compute_dispatch(document: &Document, label: &str) {
    let handle = handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_compute_dispatch",
        format!("gpu_compute_dispatch sketch handle={handle} (needs GPU)"),
        json!({
            "handle": handle,
            "wgsl": TINY_WGSL,
            "entry": "main",
            "bindings": [{ "binding": 0, "kind": "storage", "data": [0, 0, 0, 0] }],
        }),
    );
}

pub(super) fn run_gpu_compute_readback(document: &Document, label: &str) {
    let handle = handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_compute_readback",
        format!("gpu_compute_readback sketch handle={handle}"),
        json!({ "handle": handle }),
    );
}

pub(super) fn run_gpu_validate_shader(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Render.gpu_validate_shader",
        "gpu_validate_shader sketch tiny WGSL".to_string(),
        json!({ "wgsl": TINY_WGSL }),
    );
}

pub(super) fn run_gpu_compile_shader(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Render.gpu_compile_shader",
        "gpu_compile_shader sketch (needs GPU)".to_string(),
        json!({ "wgsl": TINY_WGSL }),
    );
}

pub(super) fn run_gpu_compile_to_glsl(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Render.gpu_compile_to_glsl",
        "gpu_compile_to_glsl sketch tiny WGSL".to_string(),
        json!({ "wgsl": TINY_WGSL }),
    );
}

pub(super) fn run_gpu_backend_info(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Render.gpu_backend_info",
        "gpu_backend_info sketch (needs GPU adapter)".to_string(),
        json!({}),
    );
}
