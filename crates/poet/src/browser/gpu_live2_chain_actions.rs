//! Dual-path Tool Chest actions for remaining already-bound `Render.gpu_*` / `Render.emf_*` (wave 35).
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

fn gpu_handle(document: &Document) -> u64 {
    numeric_attr(selected_container(document).as_ref(), "data-gpu-handle")
        .and_then(|v| (v >= 0.0).then_some(v as u64))
        .unwrap_or(0)
}

pub(super) fn run_gpu_upload_mesh_colored(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_upload_mesh_colored",
        format!("gpu_upload_mesh_colored sketch unit triangle handle={handle}"),
        json!({
            "handle": handle,
            "positions": [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            "colors": [1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0],
            "indices": [0, 1, 2],
        }),
    );
}

pub(super) fn run_gpu_set_standpoint(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_set_standpoint",
        format!("gpu_set_standpoint sketch spectator handle={handle}"),
        json!({ "handle": handle, "class": "spectator", "epistemic_q": 1.0, "t_slice": 0.5 }),
    );
}

pub(super) fn run_gpu_observer_standpoint(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_observer_standpoint",
        format!("gpu_observer_standpoint sketch handle={handle}"),
        json!({ "handle": handle }),
    );
}

pub(super) fn run_gpu_camera_state(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_camera_state",
        format!("gpu_camera_state sketch handle={handle}"),
        json!({ "handle": handle }),
    );
}

pub(super) fn run_gpu_surface_size(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_surface_size",
        format!("gpu_surface_size sketch handle={handle}"),
        json!({ "handle": handle }),
    );
}

pub(super) fn run_gpu_has_mesh(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_has_mesh",
        format!("gpu_has_mesh sketch handle={handle}"),
        json!({ "handle": handle }),
    );
}

pub(super) fn run_gpu_has_tensor(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_has_tensor",
        format!("gpu_has_tensor sketch handle={handle}"),
        json!({ "handle": handle }),
    );
}

pub(super) fn run_gpu_tensor_node_count(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_tensor_node_count",
        format!("gpu_tensor_node_count sketch handle={handle}"),
        json!({ "handle": handle }),
    );
}

pub(super) fn run_gpu_particle_count(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_particle_count",
        format!("gpu_particle_count sketch handle={handle}"),
        json!({ "handle": handle }),
    );
}

pub(super) fn run_gpu_sync_bloom(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_sync_bloom",
        format!("gpu_sync_bloom sketch handle={handle} (needs GPU)"),
        json!({ "handle": handle }),
    );
}

pub(super) fn run_gpu_set_artefact_joint(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_set_artefact_joint",
        format!("gpu_set_artefact_joint sketch revolute handle={handle}"),
        json!({ "handle": handle, "kind": "revolute", "axis": [0.0, 1.0, 0.0], "rate": 1.0 }),
    );
}

pub(super) fn run_gpu_set_artefact_world(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_set_artefact_world",
        format!("gpu_set_artefact_world sketch unit AABB handle={handle}"),
        json!({ "handle": handle, "min": [-1.0, -1.0, -1.0], "max": [1.0, 1.0, 1.0] }),
    );
}

pub(super) fn run_gpu_artefact_refused(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_artefact_refused",
        format!("gpu_artefact_refused sketch handle={handle}"),
        json!({ "handle": handle }),
    );
}

pub(super) fn run_gpu_required_rgba8_bytes(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.gpu_required_rgba8_bytes",
        format!("gpu_required_rgba8_bytes sketch handle={handle}"),
        json!({ "handle": handle }),
    );
}

pub(super) fn run_emf_upload_field(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.emf_upload_field",
        format!("emf_upload_field sketch 1×1×1 cell handle={handle} (needs GPU)"),
        json!({
            "handle": handle,
            "nx": 1, "ny": 1, "nz": 1, "nt": 1,
            "cells": [{ "amplitude": 1.0, "phase": 0.0, "frequency": 1.0, "scale": 1.0 }],
            "bounds": [-1.0, 1.0, -1.0, 1.0, -1.0, 1.0],
        }),
    );
}

pub(super) fn run_emf_render_slice(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.emf_render_slice",
        format!("emf_render_slice sketch handle={handle} (needs GPU)"),
        json!({ "handle": handle, "slice_z": 0, "slice_t": 0 }),
    );
}

pub(super) fn run_emf_field_info(document: &Document, label: &str) {
    let handle = gpu_handle(document);
    invoke_dual(
        document,
        label,
        "Render.emf_field_info",
        format!("emf_field_info sketch handle={handle}"),
        json!({ "handle": handle }),
    );
}
