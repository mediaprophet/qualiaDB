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
    let _ = container.set_attribute("data-spec-tool", tool.id);
    let _ = container.set_attribute("data-spec-chain", tool.chain);
    if let Some(css) = local_css(tool.id) {
        if let Ok(el) = container.clone().dyn_into::<HtmlElement>() {
            if let Some((property, value)) = css.split_once(':') {
                let _ = el.style().set_property(property.trim(), value.trim());
            }
        }
    }
    super::super::history::push_current_frame(tool.id);
    super::super::interactions::show_tool_status(document, label, tool.tooltip, "success");
}

fn apply_live(document: &Document, tool: &SpecTool, label: &str, capability: &'static str) {
    if let Some(container) = selected_container(document) {
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
            "success",
        );
        return;
    }
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
        let args = serde_json::json!({});
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

fn local_css(tool_id: &str) -> Option<&'static str> {
    match tool_id.rsplit(':').next().unwrap_or(tool_id) {
        "underline" => Some("text-decoration: underline"),
        "strikethrough" => Some("text-decoration: line-through"),
        "subscript" => Some("vertical-align: sub"),
        "superscript" => Some("vertical-align: super"),
        "highlight" => Some("background: color-mix(in srgb, var(--accent-amber) 35%, transparent)"),
        "gaussian-blur" => Some("filter: blur(2px)"),
        "sharpen" => Some("filter: contrast(1.15)"),
        "hue-saturation" => Some("filter: saturate(1.2)"),
        _ => None,
    }
}
