//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Local instrument commands, document edits, and daemon invoke sessions.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

pub(super) fn local_instrument_action(tool_id: &str) -> bool {
    matches!(
        tool_id,
        "doc:bold"
            | "doc:italic"
            | "doc:code"
            | "doc:entity"
            | "doc:objective"
            | "doc:subjective"
            | "doc:view-md"
            | "doc:view-rdf"
            | "code:run"
            | "graph:sparql"
            | "epi:objective"
            | "epi:subjective"
            | "epi:inter"
            | "epi:normative"
            | "office:doc"
            | "office:ont"
            | "office:slide"
            | "img:media"
            | "img:marker"
            | "sheet:place"
            | "sheet:fx"
            | "sheet:sum"
            | "sheet:avg"
            | "ont:add-row"
            | "ont:shacl"
            | "ont:classes"
            | "ont:export"
            | "social:connect"
            | "social:chat"
            | "social:agent"
            | "social:graph"
            | "graph:expand"
            | "graph:collapse"
            | "graph:layout"
            | "map:pin"
            | "3d:orbit"
            | "3d:pan"
            | "3d:zoom"
            | "3d:wireframe"
            | "health:biomarker"
            | "health:tomography"
            | "health:anatomy"
            | "code:ast"
            | "code:pulse"
            | "code:cap"
            | "rights:sign"
            | "rights:audit"
            | "rights:consent"
            | "webview:clip"
            | "spatial:map"
            | "spatial:3d"
            | "spatial:pin"
            | "comm:social"
            | "comm:webrtc"
            | "comm:webview"
            | "rights:group"
            | "health:place"
            | "health:anat"
            | "code:vibe"
            | "ai:extractor"
            | "ai:sentinel"
            | "ai:triad"
            | "scene:create"
            | "render:gpu_adapter"
            | "audio:transport_play"
            | "audio:transport_stop"
            | "audio:oscillator"
            | "health:nlp_ingest"
    )
}

pub(super) fn instrument_requires_daemon(tool_id: &str) -> bool {
    matches!(
        tool_id,
        "code:run"
            | "graph:sparql"
            | "ai:extractor"
            | "ai:sentinel"
            | "scene:create"
            | "render:gpu_adapter"
            | "audio:transport_play"
            | "audio:transport_stop"
            | "audio:oscillator"
            | "health:nlp_ingest"
            | "sheet:sum"
            | "sheet:avg"
            | "ont:shacl"
            | "ont:classes"
            | "ont:export"
            | "social:connect"
            | "social:chat"
            | "social:agent"
            | "social:graph"
            | "graph:expand"
            | "graph:collapse"
            | "graph:layout"
            | "3d:orbit"
            | "3d:pan"
            | "3d:zoom"
            | "3d:wireframe"
            | "health:biomarker"
            | "health:tomography"
            | "rights:sign"
            | "rights:audit"
            | "webview:clip"
    )
}

pub(super) fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

pub(super) fn click_selected(document: &Document, selector: &str, label: &str) {
    let target = selected_container(document)
        .and_then(|container| container.query_selector(selector).ok().flatten());
    if let Some(target) = target.and_then(|element| element.dyn_into::<HtmlElement>().ok()) {
        target.click();
    } else {
        crate::browser::interactions::show_tool_status(
            document,
            label,
            "The selected container does not expose this operation.",
            "error",
        );
    }
}

pub(super) fn exec_document_command(
    document: &Document,
    command: &str,
    value: Option<&str>,
    label: &str,
) {
    let result = document
        .clone()
        .dyn_into::<web_sys::HtmlDocument>()
        .ok()
        .map(|html| match value {
            Some(value) => html.exec_command_with_show_ui_and_value(command, false, value),
            None => html.exec_command(command),
        });
    match result {
        Some(Ok(true)) => {
            crate::browser::history::push_current_frame("format document");
            crate::browser::interactions::show_tool_status(
                document,
                label,
                "Applied to the active document selection.",
                "success",
            );
        }
        _ => crate::browser::interactions::show_tool_status(
            document,
            label,
            "Focus a document editor and select text before applying this operation.",
            "error",
        ),
    }
}

pub(super) fn insert_into_editor(document: &Document, snippet: &str, label: &str) {
    let editor = selected_container(document)
        .and_then(|container| container.query_selector(".vibe-editor").ok().flatten());
    if let Some(editor) = editor {
        let current = editor.text_content().unwrap_or_default();
        let next = if current.trim().is_empty() {
            snippet.to_string()
        } else {
            format!("{current}\n{snippet}")
        };
        editor.set_text_content(Some(&next));
        crate::browser::history::push_current_frame("insert vibe snippet");
        crate::browser::interactions::show_tool_status(
            document,
            label,
            "Inserted into the Vibe editor.",
            "success",
        );
    } else {
        crate::browser::interactions::show_tool_status(
            document,
            label,
            "Select a code/Vibe container with an editor first.",
            "error",
        );
    }
}

pub(super) fn sheet_grid_args(document: &Document) -> serde_json::Value {
    let mut grid = Vec::new();
    let root = selected_container(document);
    for row in 1..=6 {
        let mut cells = Vec::new();
        for col in ['A', 'B', 'C', 'D', 'E'] {
            let selector = format!("[data-cell-ref=\"{col}{row}\"]");
            let text = root
                .as_ref()
                .and_then(|container| container.query_selector(&selector).ok().flatten())
                .and_then(|cell| cell.text_content())
                .unwrap_or_default();
            cells.push(text.trim().parse::<f64>().unwrap_or(0.0));
        }
        grid.push(cells);
    }
    serde_json::json!({ "grid": grid, "range": "A1:E6" })
}

pub(super) fn invoke_session(
    document: &Document,
    label: &str,
    capability: &'static str,
    args: serde_json::Value,
) {
    if !crate::browser::native_daemon::is_daemon_connected() {
        crate::browser::interactions::show_tool_status(
            document,
            label,
            "Unavailable: start the local QualiaDB daemon.",
            "unavailable",
        );
        return;
    }
    crate::browser::interactions::show_tool_status(
        document,
        label,
        &format!("Running {capability}…"),
        "running",
    );
    let label = label.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match crate::browser::native_daemon::daemon_invoke(capability, args).await {
            Ok(response) if response.ok => crate::browser::interactions::show_tool_status(
                &document,
                &label,
                &response.value,
                "success",
            ),
            Ok(response) => crate::browser::interactions::show_tool_status(
                &document,
                &label,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or("Native session invoke failed."),
                "error",
            ),
            Err(error) => {
                crate::browser::interactions::show_tool_status(&document, &label, &error, "error")
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{instrument_requires_daemon, local_instrument_action};

    #[test]
    fn standalone_doc_bold_is_local() {
        assert!(local_instrument_action("doc:bold"));
        assert!(!local_instrument_action("not-a-tool"));
    }

    #[test]
    fn sparql_requires_the_daemon() {
        assert!(instrument_requires_daemon("graph:sparql"));
        assert!(!instrument_requires_daemon("doc:bold"));
    }
}
