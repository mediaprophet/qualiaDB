//! Dual-path Tool Chest actions for Host-bound `HbbTV.*` ids.

use serde_json::json;
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
    let bounded: String = text.chars().take(4_096).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

fn string_attr(el: Option<&Element>, name: &str) -> Option<String> {
    el.and_then(|e| e.get_attribute(name))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
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

fn app_id(document: &Document) -> String {
    string_attr(selected_container(document).as_ref(), "data-hbbtv-id")
        .unwrap_or_else(|| "app-1".into())
}

fn first_line(document: &Document) -> String {
    selected_source(document)
        .and_then(|s| {
            s.lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .map(|s| s.to_string())
        })
        .unwrap_or_default()
}

pub(super) fn run_new_app(document: &Document, label: &str) {
    let id = app_id(document);
    let name = first_line(document);
    let name = if name.is_empty() {
        "App".to_string()
    } else {
        name
    };
    invoke_dual(
        document,
        label,
        "HbbTV.new_app",
        format!("new_app sketch id={id} name={name}"),
        json!({ "id": id, "name": name }),
    );
}

pub(super) fn run_add_page(document: &Document, label: &str) {
    let id = app_id(document);
    let title = first_line(document);
    let title = if title.is_empty() {
        "Home".to_string()
    } else {
        title
    };
    invoke_dual(
        document,
        label,
        "HbbTV.add_page",
        format!("add_page sketch id={id} title={title}"),
        json!({ "id": id, "title": title }),
    );
}

pub(super) fn run_navigate(document: &Document, label: &str) {
    let id = app_id(document);
    let page_id = string_attr(selected_container(document).as_ref(), "data-page-id")
        .unwrap_or_else(|| "page-1".into());
    invoke_dual(
        document,
        label,
        "HbbTV.navigate",
        format!("navigate sketch id={id} page={page_id}"),
        json!({ "id": id, "page_id": page_id }),
    );
}

pub(super) fn run_set_state(document: &Document, label: &str) {
    let id = app_id(document);
    let key = string_attr(selected_container(document).as_ref(), "data-key")
        .unwrap_or_else(|| "focus".into());
    let value = first_line(document);
    let value = if value.is_empty() {
        "home".to_string()
    } else {
        value
    };
    invoke_dual(
        document,
        label,
        "HbbTV.set_state",
        format!("set_state sketch {key}={value}"),
        json!({ "id": id, "key": key, "value": value }),
    );
}
