//! Dispatch for spec tool rows. Keep this file to place / local / live / gated.
//! Do not add per-tool branches here as the swarm grows — encode behaviour on
//! the row (`Contract`) or in a small helper.

use super::row::{Contract, SpecTool};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

pub fn gated_reason(tool: &SpecTool) -> Option<&'static str> {
    match tool.contract {
        Contract::Gated(reason) | Contract::Parked(reason) => Some(reason),
        _ => None,
    }
}

pub fn run(document: &Document, tool: &SpecTool, label: &str) {
    match tool.contract {
        Contract::Gated(reason) | Contract::Parked(reason) => {
            super::super::interactions::show_tool_status(document, label, reason, "unavailable");
        }
        Contract::Place(container_type) => {
            super::super::interactions::place_container_via_menu(document, container_type, label);
        }
        Contract::Local => apply_local(document, tool, label),
        Contract::Live(capability) => apply_live(document, tool, label, capability),
    }
}

fn apply_local(document: &Document, tool: &SpecTool, label: &str) {
    let Some(container) = selected_container(document) else {
        super::super::interactions::show_tool_status(
            document,
            label,
            "Select a page first, then use this tool.",
            "error",
        );
        return;
    };
    if super::media_actions::run(document, &container, tool.id) {
        return;
    }
    let result = if let Some(result) = super::office_actions::run(document, &container, tool.id) {
        result.map(|()| true)
    } else if let Some(result) = super::epistemic_actions::run(document, &container, tool.id) {
        result.map(|()| true)
    } else if let Some(result) = super::ai_actions::run(document, &container, tool.id) {
        result.map(|()| true)
    } else if let Some(result) = super::investigation_actions::run(document, &container, tool.id) {
        result.map(|()| true)
    } else if let Some(result) = super::research_actions::run(document, &container, tool.id) {
        result.map(|()| true)
    } else if let Some(result) = super::code_actions::run(document, &container, tool.id) {
        result.map(|()| true)
    } else if let Some(result) = super::image_actions::run(document, &container, tool.id) {
        result.map(|()| true)
    } else if let Some(result) = super::video_actions::run(document, &container, tool.id) {
        result.map(|()| true)
    } else if let Some(result) = super::audio_actions::run(document, &container, tool.id) {
        result.map(|()| true)
    } else if let Some(result) = super::spatial3d_actions::run(document, &container, tool.id) {
        result.map(|()| true)
    } else if let Some(result) = super::productions_actions::run(document, &container, tool.id) {
        result.map(|()| true)
    } else if let Some(result) = super::portals_actions::run(document, &container, tool.id) {
        result.map(|()| true)
    } else if let Some(result) = super::hypermedia_actions::run(document, &container, tool.id) {
        result.map(|()| true)
    } else if let Ok(element) = container.clone().dyn_into::<HtmlElement>() {
        super::local_effects::apply(document, &element, tool.id).map_err(str::to_string)
    } else {
        Err("This surface does not support this action.".to_string())
    };
    match result {
        Err(reason) => {
            super::super::interactions::show_tool_status(document, label, &reason, "error");
            return;
        }
        Ok(false) => {
            // Legacy Local rows select a tool; they do not execute its named operation.
            let _ = container.set_attribute("data-spec-tool", tool.id);
            let _ = container.set_attribute("data-spec-chain", tool.chain);
            super::super::interactions::show_tool_status(
                document, label,
                "Tool selected. Its editing action is not implemented on this surface yet.",
                "unavailable",
            );
            return;
        }
        Ok(true) => {}
    }
    let _ = container.set_attribute("data-spec-tool", tool.id);
    let _ = container.set_attribute("data-spec-chain", tool.chain);
    super::super::history::push_current_frame(tool.id);
    super::super::interactions::show_tool_status(document, label, tool.tooltip, "success");
}

fn apply_live(document: &Document, tool: &SpecTool, label: &str, capability: &'static str) {
    let selected = selected_container(document);
    if let Some(container) = selected.as_ref() {
        let _ = container.set_attribute("data-spec-tool", tool.id);
        let _ = container.set_attribute("data-live-capability", capability);
    }
    let label = label.to_string();
    if !super::super::native_daemon::is_daemon_connected() {
        super::super::interactions::show_tool_status(
            document,
            &label,
            &format!(
                "{}. The live step needs the local QualiaDB daemon ({capability}).",
                tool.tooltip
            ),
            "unavailable",
        );
        return;
    }
    let args = match super::live_args::build(selected.as_ref(), tool, capability) {
        Ok(args) => args,
        Err(reason) => {
            super::super::interactions::show_tool_status(document, &label, &reason, "unavailable");
            return;
        }
    };
    super::super::interactions::show_tool_status(
        document,
        &label,
        &format!("Running {capability}…"),
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::super::native_daemon::daemon_invoke(capability, args).await {
            Ok(response) if response.ok => super::super::interactions::show_tool_status(
                &document,
                &label,
                &response.value,
                "success",
            ),
            Ok(response) => super::super::interactions::show_tool_status(
                &document,
                &label,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or("Live step failed."),
                "error",
            ),
            Err(error) => {
                super::super::interactions::show_tool_status(&document, &label, &error, "error")
            }
        }
    });
}
