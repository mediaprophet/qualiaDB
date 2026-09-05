//! Contextual instrument panel — dynamic toolbar that changes based on the
//! selected container type. Appears above the canvas when a container is
//! selected, hides when the canvas is clicked.
//!
//! Ports the contextual instrument panel concept from the Canvas_Workbench mockup.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event, HtmlElement};

/// Show or replace the contextual instrument panel for the given container element.
/// The instrument panel appears between the control bar and the canvas workspace.
pub fn show_for_container(document: &Document, container: &Element) {
    let container_type = container
        .get_attribute("data-container-type")
        .unwrap_or_default();

    let tools = tools_for_type(&container_type);
    if tools.is_empty() {
        hide(document);
        return;
    }

    // Remove existing instrument panel
    hide(document);

    let panel = document.create_element("div").unwrap();
    panel.set_class_name("contextual-instrument-panel");
    super::surface_aspects::mark(&panel, "dwell");
    panel
        .set_attribute("data-container-type", &container_type)
        .unwrap();

    // Context label
    let label = document.create_element("span").unwrap();
    label.set_class_name("instrument-panel-context-label");
    label.set_text_content(Some(&format!("\u{1F4CB} {} tools", container_type)));
    panel.append_child(&label).unwrap();

    // Tool buttons
    for tool in tools {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("instrument-panel-tool-btn");
        btn.set_attribute("data-tool", tool.id).unwrap();
        btn.set_attribute("title", tool.description).unwrap();
        configure_tool_button(&btn, &tool);

        let icon = document.create_element("span").unwrap();
        icon.set_class_name("instrument-panel-tool-icon");
        icon.set_text_content(Some(tool.icon));
        btn.append_child(&icon).unwrap();

        let label = document.create_element("span").unwrap();
        label.set_class_name("instrument-panel-tool-label");
        label.set_text_content(Some(tool.label));
        btn.append_child(&label).unwrap();

        panel.append_child(&btn).unwrap();
    }

    // Close button
    let close = document.create_element("button").unwrap();
    close.set_class_name("instrument-panel-close-btn");
    close.set_text_content(Some("\u{2715}"));
    panel.append_child(&close).unwrap();

    // Insert instrument panel between control bar and workspace
    if let Some(workspace) = document.query_selector(".main-workspace").unwrap() {
        workspace
            .parent_element()
            .unwrap()
            .insert_before(&panel, Some(&workspace))
            .unwrap();
    }

    wire_instrument_panel(document);
}

/// Hide the contextual instrument panel.
pub fn hide(document: &Document) {
    if let Some(existing) = document
        .query_selector(".contextual-instrument-panel")
        .unwrap()
    {
        existing.remove();
    }
}

fn local_instrument_action(tool_id: &str) -> bool {
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

fn instrument_requires_daemon(tool_id: &str) -> bool {
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

fn configure_tool_button(button: &Element, tool: &RibbonTool) {
    if !local_instrument_action(tool.id) {
        button.set_attribute("disabled", "").unwrap();
        button.set_attribute("aria-disabled", "true").unwrap();
        button
            .set_attribute(
                "title",
                &format!(
                    "Unavailable in standalone POET: {} requires a dedicated typed runtime contract.",
                    tool.description
                ),
            )
            .unwrap();
        button.set_attribute("data-honesty", "unavailable").unwrap();
    } else if instrument_requires_daemon(tool.id) {
        button
            .set_attribute("data-requires-daemon", "true")
            .unwrap();
        button
            .set_attribute("data-enabled-title", tool.description)
            .unwrap();
        if !super::native_daemon::is_daemon_connected() {
            button.set_attribute("disabled", "").unwrap();
            button.set_attribute("aria-disabled", "true").unwrap();
            button
                .set_attribute(
                    "title",
                    "Unavailable until the local QualiaDB daemon is connected.",
                )
                .unwrap();
        }
    }
}

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn click_selected(document: &Document, selector: &str, label: &str) {
    let target = selected_container(document)
        .and_then(|container| container.query_selector(selector).ok().flatten());
    if let Some(target) = target.and_then(|element| element.dyn_into::<HtmlElement>().ok()) {
        target.click();
    } else {
        super::interactions::show_tool_status(
            document,
            label,
            "The selected container does not expose this operation.",
            "error",
        );
    }
}

fn exec_document_command(document: &Document, command: &str, value: Option<&str>, label: &str) {
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
            super::history::push_current_frame("format document");
            super::interactions::show_tool_status(
                document,
                label,
                "Applied to the active document selection.",
                "success",
            );
        }
        _ => super::interactions::show_tool_status(
            document,
            label,
            "Focus a document editor and select text before applying this operation.",
            "error",
        ),
    }
}

fn insert_into_editor(document: &Document, snippet: &str, label: &str) {
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
        super::history::push_current_frame("insert vibe snippet");
        super::interactions::show_tool_status(
            document,
            label,
            "Inserted into the Vibe editor.",
            "success",
        );
    } else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a code/Vibe container with an editor first.",
            "error",
        );
    }
}

fn sheet_grid_args(document: &Document) -> serde_json::Value {
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

fn invoke_session(
    document: &Document,
    label: &str,
    capability: &'static str,
    args: serde_json::Value,
) {
    if !super::native_daemon::is_daemon_connected() {
        super::interactions::show_tool_status(
            document,
            label,
            "Unavailable: start the local QualiaDB daemon.",
            "unavailable",
        );
        return;
    }
    super::interactions::show_tool_status(
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
        match super::native_daemon::daemon_invoke(capability, args).await {
            Ok(response) if response.ok => {
                super::interactions::show_tool_status(&document, &label, &response.value, "success")
            }
            Ok(response) => super::interactions::show_tool_status(
                &document,
                &label,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or("Native session invoke failed."),
                "error",
            ),
            Err(error) => super::interactions::show_tool_status(&document, &label, &error, "error"),
        }
    });
}

fn dispatch_instrument_action(document: &Document, tool_id: &str, label: &str) {
    match tool_id {
        "doc:bold" => exec_document_command(document, "bold", None, label),
        "doc:italic" => exec_document_command(document, "italic", None, label),
        "doc:code" => exec_document_command(
            document,
            "insertHTML",
            Some("<code class=\"cml-code\">code</code>"),
            label,
        ),
        "doc:entity" => exec_document_command(
            document,
            "insertHTML",
            Some(
                "<q-entity category=\"entity\" iri=\"did:qualia:entity#term\" class=\"cml-entity\">Tagged Entity</q-entity>",
            ),
            label,
        ),
        "doc:objective" | "epi:objective" => super::tool_actions::dispatch(
            document,
            "epistemic:tag_objective",
            label,
            crate::tool_chest::core::intent_bus::ActionType::Annotate,
        ),
        "doc:subjective" | "epi:subjective" => super::tool_actions::dispatch(
            document,
            "epistemic:tag_subjective",
            label,
            crate::tool_chest::core::intent_bus::ActionType::Annotate,
        ),
        "epi:inter" => super::tool_actions::dispatch(
            document,
            "epistemic:tag_intersubjective",
            label,
            crate::tool_chest::core::intent_bus::ActionType::Annotate,
        ),
        "epi:normative" => super::tool_actions::dispatch(
            document,
            "epistemic:tag_normative",
            label,
            crate::tool_chest::core::intent_bus::ActionType::Annotate,
        ),
        "doc:view-md" => {
            click_selected(document, ".doc-view-tab[data-doc-view=\"markdown\"]", label)
        }
        "doc:view-rdf" => click_selected(document, ".doc-view-tab[data-doc-view=\"rdf\"]", label),
        "code:run" | "graph:sparql" | "health:nlp_ingest" => click_selected(
            document,
            &format!("[data-instrument-action=\"{tool_id}\"]"),
            label,
        ),
        "office:doc" => super::interactions::place_container_via_menu(document, "doc", label),
        "office:ont" => super::interactions::place_container_via_menu(document, "ontology", label),
        "office:slide" => super::interactions::place_container_via_menu(document, "slide", label),
        "img:media" => super::interactions::place_container_via_menu(document, "media", label),
        "sheet:place" => super::interactions::place_container_via_menu(document, "sheet", label),
        "spatial:map" => super::interactions::place_container_via_menu(document, "map", label),
        "spatial:3d" => super::interactions::place_container_via_menu(document, "3d", label),
        "comm:social" => super::interactions::place_container_via_menu(document, "social", label),
        "comm:webrtc" => super::interactions::place_container_via_menu(document, "webrtc", label),
        "comm:webview" => super::interactions::place_container_via_menu(document, "webview", label),
        "health:place" => super::interactions::place_container_via_menu(document, "health", label),
        "health:anat" => super::interactions::place_container_via_menu(document, "anatomy", label),
        "code:vibe" => super::interactions::place_container_via_menu(document, "code", label),
        "ai:triad" => super::interactions::place_container_via_menu(document, "triad", label),
        "img:marker" => super::tool_actions::dispatch(
            document,
            "image:marker",
            label,
            crate::tool_chest::core::intent_bus::ActionType::Annotate,
        ),
        "spatial:pin" => super::tool_actions::dispatch(
            document,
            "spatial:pin",
            label,
            crate::tool_chest::core::intent_bus::ActionType::Annotate,
        ),
        "rights:group" => super::tool_actions::dispatch(
            document,
            "rights:authors_group",
            label,
            crate::tool_chest::core::intent_bus::ActionType::Invoke,
        ),
        "ai:extractor" | "ai:sentinel" => super::tool_actions::dispatch(
            document,
            tool_id,
            label,
            crate::tool_chest::core::intent_bus::ActionType::Invoke,
        ),
        "scene:create" => invoke_session(
            document,
            label,
            "Scene.create",
            serde_json::json!({ "name": "poet-scene-session" }),
        ),
        "render:gpu_adapter" => invoke_session(
            document,
            label,
            "Render.gpu_adapter_info",
            serde_json::json!({}),
        ),
        "audio:transport_play" => invoke_session(
            document,
            label,
            "Audio.transport",
            serde_json::json!({ "action": "play", "tempo": 120.0 }),
        ),
        "audio:transport_stop" => invoke_session(
            document,
            label,
            "Audio.transport",
            serde_json::json!({ "action": "stop", "tempo": 120.0 }),
        ),
        "audio:oscillator" => invoke_session(
            document,
            label,
            "Audio.oscillator",
            serde_json::json!({
                "waveform": "sine",
                "frequency": 440.0,
                "sample_rate": 44100.0,
                "n": 512
            }),
        ),
        "sheet:fx" => {
            if let Some(input) = selected_container(document).and_then(|container| {
                container
                    .query_selector(".vibe-toolbar input")
                    .ok()
                    .flatten()
                    .and_then(|element| element.dyn_into::<HtmlElement>().ok())
            }) {
                let _ = input.focus();
                super::interactions::show_tool_status(
                    document,
                    label,
                    "Formula bar focused. Enter =SUM(A1:A10) in a cell.",
                    "success",
                );
            } else {
                super::interactions::show_tool_status(
                    document,
                    label,
                    "Select a sheet container first.",
                    "error",
                );
            }
        }
        "sheet:sum" => invoke_session(
            document,
            label,
            "Sheet.sum_range",
            sheet_grid_args(document),
        ),
        "sheet:avg" => invoke_session(document, label, "Sheet.stats", sheet_grid_args(document)),
        "ont:add-row" => click_selected(document, "[data-cop-family] button", label),
        "ont:shacl" => invoke_session(document, label, "SHACL.extensions", serde_json::json!({})),
        "ont:classes" => invoke_session(
            document,
            label,
            "GraphDatabase.stats",
            serde_json::json!({}),
        ),
        "ont:export" => invoke_session(
            document,
            label,
            "GraphAuthoring.process",
            serde_json::json!({
                "source": crate::browser::ontology_views::persist::PERSON_SAFE_N3,
                "mode": "ontology_compile",
                "format": "turtle"
            }),
        ),
        "social:connect" => invoke_session(
            document,
            label,
            "Pulse.publish_notification",
            serde_json::json!({ "channel": "poet/social-requests" }),
        ),
        "social:chat" => invoke_session(
            document,
            label,
            "Pulse.publish",
            serde_json::json!({ "channel": "poet/social", "payload_type": "agent-message" }),
        ),
        "social:agent" => invoke_session(
            document,
            label,
            "Pulse.publish_agent_message",
            serde_json::json!({ "channel": "poet/social" }),
        ),
        "social:graph" | "graph:expand" | "graph:collapse" | "graph:layout" => invoke_session(
            document,
            label,
            "GraphDatabase.stats",
            serde_json::json!({}),
        ),
        "map:pin" => super::tool_actions::dispatch(
            document,
            "spatial:pin",
            label,
            crate::tool_chest::core::intent_bus::ActionType::Annotate,
        ),
        "3d:orbit" | "3d:pan" | "3d:zoom" | "3d:wireframe" => invoke_session(
            document,
            label,
            "Render.gpu_adapter_info",
            serde_json::json!({}),
        ),
        "health:biomarker" => super::interactions::place_container_via_menu(
            document,
            "health_calculators",
            label,
        ),
        "health:tomography" => invoke_session(
            document,
            label,
            "MedicalImaging.hu_window",
            serde_json::json!({
                "study_uid": "urn:poet:anatomy:demo-slice",
                "width": 2,
                "height": 2,
                "pixels": [-160.0, 40.0, 240.0, 1000.0],
                "window": 400.0,
                "level": 40.0
            }),
        ),
        "health:anatomy" => {
            super::interactions::place_container_via_menu(document, "anatomy", label)
        }
        "code:ast" => click_selected(document, "[data-instrument-action=\"code:run\"]", label),
        "code:pulse" => {
            insert_into_editor(document, "pulse::emit(\"poet/topic\", \"payload\")", label)
        }
        "code:cap" => insert_into_editor(
            document,
            "capability.invoke(\"Poet.manifold_create\", { label: \"New lens\", nest: true })",
            label,
        ),
        "rights:sign" | "rights:audit" => invoke_session(
            document,
            label,
            "DeonticLogic.evaluate",
            serde_json::json!({ "modality": "obligate", "body": "rights" }),
        ),
        "rights:consent" => click_selected(document, "[data-cop-family] button", label),
        "webview:clip" => invoke_session(
            document,
            label,
            "Document.ingest",
            serde_json::json!({ "text": "sandbox navigation record", "uri": "urn:poet:webview" }),
        ),
        _ => super::interactions::show_tool_status(
            document,
            label,
            "Unavailable: this instrument has no registered standalone runtime contract.",
            "unavailable",
        ),
    }
}

/// Wire instrument panel button clicks and the close button.
fn wire_instrument_panel(document: &Document) {
    // Tool buttons
    let buttons = document
        .query_selector_all(".instrument-panel-tool-btn")
        .unwrap();
    for i in 0..buttons.length() {
        let btn = buttons.get(i).unwrap();
        let btn_el: Element = btn.dyn_into().unwrap();
        let tool_id = btn_el.get_attribute("data-tool").unwrap_or_default();
        let label = btn_el
            .query_selector(".instrument-panel-tool-label")
            .unwrap()
            .map(|el| el.text_content().unwrap_or_default())
            .unwrap_or_default();

        let closure = Closure::wrap(Box::new(move |_e: Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            dispatch_instrument_action(&doc, &tool_id, &label);
        }) as Box<dyn FnMut(Event)>);

        btn_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Close button
    if let Some(close) = document
        .query_selector(".instrument-panel-close-btn")
        .unwrap()
    {
        let closure = Closure::wrap(Box::new(move |_e: Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            hide(&doc);
        }) as Box<dyn FnMut(Event)>);
        close
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

// ---------------------------------------------------------------------------
// Tool definitions per container type
// ---------------------------------------------------------------------------

struct RibbonTool {
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    description: &'static str,
}

fn tools_for_type(container_type: &str) -> Vec<RibbonTool> {
    match container_type {
        "doc" => vec![
            RibbonTool {
                id: "doc:bold",
                icon: "B",
                label: "Bold",
                description: "Bold selected text",
            },
            RibbonTool {
                id: "doc:italic",
                icon: "I",
                label: "Italic",
                description: "Italicize selected text",
            },
            RibbonTool {
                id: "doc:code",
                icon: "</>",
                label: "Code",
                description: "Format as code",
            },
            RibbonTool {
                id: "doc:entity",
                icon: "\u{1F3F7}",
                label: "Entity",
                description: "Tag as RDF entity <q-entity>",
            },
            RibbonTool {
                id: "doc:objective",
                icon: "\u{1F52C}",
                label: "Objective",
                description: "Tag as objective epistemic modality",
            },
            RibbonTool {
                id: "doc:subjective",
                icon: "\u{1F9E0}",
                label: "Subjective",
                description: "Tag as subjective qualia",
            },
            RibbonTool {
                id: "doc:view-md",
                icon: "MD",
                label: "Markdown",
                description: "Switch to markdown source view",
            },
            RibbonTool {
                id: "doc:view-rdf",
                icon: "RDF",
                label: "Triples",
                description: "Switch to RDF triples view",
            },
        ],
        "sheet" => vec![
            RibbonTool {
                id: "sheet:fx",
                icon: "fx",
                label: "Formula",
                description: "Formula bar",
            },
            RibbonTool {
                id: "sheet:sum",
                icon: "\u{03A3}",
                label: "Sum",
                description: "Sum selected cells",
            },
            RibbonTool {
                id: "sheet:avg",
                icon: "\u{00B5}",
                label: "Average",
                description: "Average selected cells",
            },
            RibbonTool {
                id: "sheet:p64",
                icon: "P64",
                label: "Latent",
                description: "EnCodec P64 neural latent",
            },
            RibbonTool {
                id: "sheet:chart",
                icon: "\u{1F4CA}",
                label: "Chart",
                description: "Insert chart",
            },
        ],
        "dual_studio" | "scene_view" => vec![
            RibbonTool {
                id: "scene:create",
                icon: "SCN",
                label: "Scene.create",
                description: "Create a Scene session on the daemon",
            },
            RibbonTool {
                id: "render:gpu_adapter",
                icon: "GPU",
                label: "GPU adapter",
                description: "Query Render.gpu_adapter_info",
            },
            RibbonTool {
                id: "audio:transport_play",
                icon: "\u{25B6}",
                label: "Play",
                description: "Audio.transport play",
            },
            RibbonTool {
                id: "audio:transport_stop",
                icon: "\u{23F9}",
                label: "Stop",
                description: "Audio.transport stop",
            },
        ],
        "health_overview" | "health_calculators" | "health_documents" | "disclosure_log"
        | "conditions" => {
            vec![RibbonTool {
                id: "health:nlp_ingest",
                icon: "NLP",
                label: "NLP ingest",
                description: "Run nlp.analyze + gazetteer + Semantic Library ingest on pasted text",
            }]
        }
        "audio_session" => vec![
            RibbonTool {
                id: "audio:transport_play",
                icon: "\u{25B6}",
                label: "Play",
                description: "Audio.transport play",
            },
            RibbonTool {
                id: "audio:transport_stop",
                icon: "\u{23F9}",
                label: "Stop",
                description: "Audio.transport stop",
            },
            RibbonTool {
                id: "audio:oscillator",
                icon: "Hz",
                label: "Oscillator",
                description: "Audio.oscillator 440 Hz sine",
            },
        ],
        "code" => vec![
            RibbonTool {
                id: "code:run",
                icon: "\u{25B6}",
                label: "Run",
                description: "Run VibeScript",
            },
            RibbonTool {
                id: "code:ast",
                icon: "AST",
                label: "AST",
                description: "Homoiconic AST inspector",
            },
            RibbonTool {
                id: "code:gas",
                icon: "\u{26FD}",
                label: "Gas",
                description: "Gas accounting",
            },
            RibbonTool {
                id: "code:pulse",
                icon: "\u{1F4A3}",
                label: "pulse::emit",
                description: "Insert pulse::emit",
            },
            RibbonTool {
                id: "code:cap",
                icon: "\u{1F511}",
                label: "capability.invoke",
                description: "Insert Poet.manifold_create (author a lens / container / subject)",
            },
        ],
        "ontology" => vec![
            RibbonTool {
                id: "ont:add-row",
                icon: "+",
                label: "Add Alignment",
                description: "Add alignment row",
            },
            RibbonTool {
                id: "ont:shacl",
                icon: "\u{2705}",
                label: "SHACL",
                description: "Validate SHACL shapes",
            },
            RibbonTool {
                id: "ont:classes",
                icon: "\u{1F3DB}",
                label: "Classes",
                description: "Browse class declarations",
            },
            RibbonTool {
                id: "ont:export",
                icon: "\u{1F4E4}",
                label: "Export",
                description: "Export ontology as TTL",
            },
        ],
        "map" => vec![
            RibbonTool {
                id: "map:pin",
                icon: "\u{1F4CD}",
                label: "Pin",
                description: "Place incident pin",
            },
            RibbonTool {
                id: "map:track",
                icon: "\u{1F50D}",
                label: "Track",
                description: "Add UAV track",
            },
            RibbonTool {
                id: "map:flow",
                icon: "\u{1F4A7}",
                label: "Flow",
                description: "Toggle flow layer",
            },
            RibbonTool {
                id: "map:trail",
                icon: "\u{1F6F6}",
                label: "Trail",
                description: "Toggle trail layer",
            },
        ],
        "social" => vec![
            RibbonTool {
                id: "social:connect",
                icon: "\u{1F91D}",
                label: "Connect",
                description: "Send connection request",
            },
            RibbonTool {
                id: "social:chat",
                icon: "\u{1F4AC}",
                label: "Chat",
                description: "New chat session",
            },
            RibbonTool {
                id: "social:agent",
                icon: "\u{1F916}",
                label: "Agent",
                description: "Add AI sub-agent",
            },
            RibbonTool {
                id: "social:graph",
                icon: "\u{1F578}",
                label: "Graph",
                description: "View chat graph",
            },
        ],
        "graph" => vec![
            RibbonTool {
                id: "graph:sparql",
                icon: "\u{1F50D}",
                label: "SPARQL",
                description: "Run SPARQL query",
            },
            RibbonTool {
                id: "graph:expand",
                icon: "\u{1F504}",
                label: "Expand",
                description: "Expand node neighbors",
            },
            RibbonTool {
                id: "graph:collapse",
                icon: "\u{1F4E5}",
                label: "Collapse",
                description: "Collapse node",
            },
            RibbonTool {
                id: "graph:layout",
                icon: "\u{1F4D0}",
                label: "Layout",
                description: "Auto-layout graph",
            },
        ],
        "media" | "3d" => vec![
            RibbonTool {
                id: "3d:orbit",
                icon: "\u{1F504}",
                label: "Orbit",
                description: "Orbit camera",
            },
            RibbonTool {
                id: "3d:pan",
                icon: "\u{270B}",
                label: "Pan",
                description: "Pan camera",
            },
            RibbonTool {
                id: "3d:zoom",
                icon: "\u{1F50D}",
                label: "Zoom",
                description: "Zoom to fit",
            },
            RibbonTool {
                id: "3d:wireframe",
                icon: "\u{1F9F1}",
                label: "Wireframe",
                description: "Toggle wireframe",
            },
        ],
        "health" => vec![
            RibbonTool {
                id: "health:biomarker",
                icon: "\u{1F52C}",
                label: "Biomarkers",
                description: "Pathology biomarker table",
            },
            RibbonTool {
                id: "health:tomography",
                icon: "\u{1F3A4}",
                label: "Tomography",
                description: "Spectral acoustic tomography",
            },
            RibbonTool {
                id: "health:anatomy",
                icon: "\u{1F9B2}",
                label: "Anatomy",
                description: "10D vocal tract resonator",
            },
        ],
        "webrtc" => vec![
            RibbonTool {
                id: "webrtc:mic",
                icon: "\u{1F3A4}",
                label: "Mic",
                description: "Toggle microphone",
            },
            RibbonTool {
                id: "webrtc:cam",
                icon: "\u{1F4F7}",
                label: "Cam",
                description: "Toggle camera",
            },
            RibbonTool {
                id: "webrtc:share",
                icon: "\u{1F4BB}",
                label: "Share",
                description: "Screen share",
            },
        ],
        "webview" => vec![
            RibbonTool {
                id: "webview:back",
                icon: "\u{2B05}",
                label: "Back",
                description: "Navigate back",
            },
            RibbonTool {
                id: "webview:forward",
                icon: "\u{27A1}",
                label: "Forward",
                description: "Navigate forward",
            },
            RibbonTool {
                id: "webview:reload",
                icon: "\u{1F503}",
                label: "Reload",
                description: "Reload page",
            },
            RibbonTool {
                id: "webview:clip",
                icon: "\u{2702}",
                label: "Clip RDF",
                description: "Extract RDF from page",
            },
        ],
        "rights" => vec![
            RibbonTool {
                id: "rights:sign",
                icon: "\u{270D}",
                label: "Sign",
                description: "DID sign document",
            },
            RibbonTool {
                id: "rights:audit",
                icon: "\u{1F4DC}",
                label: "Audit",
                description: "Audit trail",
            },
            RibbonTool {
                id: "rights:consent",
                icon: "\u{2705}",
                label: "Consent",
                description: "Set consent",
            },
        ],
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// Tool-chain activation (OSX menu behaviour)
// ---------------------------------------------------------------------------

/// Activate a tool-chain on the currently focused surface (container or manifold).
/// If a container is selected, the chain's tools appear in the instrument panel for that container.
/// If no container is selected, the chain's tools appear as manifold-level tools.
pub fn activate_chain(document: &Document, chain_id: &str) {
    let tools = tools_for_chain(chain_id);
    if tools.is_empty() {
        return;
    }

    hide(document);

    let panel = document.create_element("div").unwrap();
    panel.set_class_name("contextual-instrument-panel");
    panel.set_attribute("data-chain-id", chain_id).unwrap();

    // Context label — shows which chain is active
    let label = document.create_element("span").unwrap();
    label.set_class_name("instrument-panel-context-label");
    label.set_text_content(Some(&format!(
        "\u{2630} {} \u{2192} focused surface",
        chain_label(chain_id)
    )));
    panel.append_child(&label).unwrap();

    // Tool buttons
    for tool in &tools {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("instrument-panel-tool-btn");
        btn.set_attribute("data-tool", tool.id).unwrap();
        btn.set_attribute("title", tool.description).unwrap();
        configure_tool_button(&btn, tool);

        let icon = document.create_element("span").unwrap();
        icon.set_class_name("instrument-panel-tool-icon");
        icon.set_text_content(Some(tool.icon));
        btn.append_child(&icon).unwrap();

        let label = document.create_element("span").unwrap();
        label.set_class_name("instrument-panel-tool-label");
        label.set_text_content(Some(tool.label));
        btn.append_child(&label).unwrap();

        panel.append_child(&btn).unwrap();
    }

    // Close button
    let close = document.create_element("button").unwrap();
    close.set_class_name("instrument-panel-close-btn");
    close.set_text_content(Some("\u{2715}"));
    panel.append_child(&close).unwrap();

    // Insert instrument panel between control bar and workspace
    if let Some(workspace) = document.query_selector(".main-workspace").unwrap() {
        workspace
            .parent_element()
            .unwrap()
            .insert_before(&panel, Some(&workspace))
            .unwrap();
    }

    wire_instrument_panel(document);
}

/// Activate a tool-chain on a specific container (via drag-drop).
/// This selects the container and shows the chain's tools in the instrument panel.
pub fn activate_chain_on_container(document: &Document, chain_id: &str) {
    // Find the selected container
    let selected = document
        .query_selector(".canvas-container-node.selected")
        .unwrap();
    let container_type = if let Some(ref el) = selected {
        el.get_attribute("data-container-type").unwrap_or_default()
    } else {
        String::new()
    };

    let tools = if container_type.is_empty() {
        tools_for_chain(chain_id)
    } else {
        // Merge container-type tools with chain tools
        let mut t = tools_for_type(&container_type);
        t.extend(tools_for_chain(chain_id));
        t
    };

    if tools.is_empty() {
        return;
    }

    hide(document);

    let panel = document.create_element("div").unwrap();
    panel.set_class_name("contextual-instrument-panel");
    panel.set_attribute("data-chain-id", chain_id).unwrap();
    if !container_type.is_empty() {
        panel
            .set_attribute("data-container-type", &container_type)
            .unwrap();
    }

    // Context label
    let label = document.create_element("span").unwrap();
    label.set_class_name("instrument-panel-context-label");
    if container_type.is_empty() {
        label.set_text_content(Some(&format!(
            "\u{2630} {} \u{2192} manifold",
            chain_label(chain_id)
        )));
    } else {
        label.set_text_content(Some(&format!(
            "\u{2630} {} \u{2192} {}",
            chain_label(chain_id),
            container_type
        )));
    }
    panel.append_child(&label).unwrap();

    // Tool buttons
    for tool in &tools {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("instrument-panel-tool-btn");
        btn.set_attribute("data-tool", tool.id).unwrap();
        btn.set_attribute("title", tool.description).unwrap();
        configure_tool_button(&btn, tool);

        let icon = document.create_element("span").unwrap();
        icon.set_class_name("instrument-panel-tool-icon");
        icon.set_text_content(Some(tool.icon));
        btn.append_child(&icon).unwrap();

        let label = document.create_element("span").unwrap();
        label.set_class_name("instrument-panel-tool-label");
        label.set_text_content(Some(tool.label));
        btn.append_child(&label).unwrap();

        panel.append_child(&btn).unwrap();
    }

    // Close button
    let close = document.create_element("button").unwrap();
    close.set_class_name("instrument-panel-close-btn");
    close.set_text_content(Some("\u{2715}"));
    panel.append_child(&close).unwrap();

    if let Some(workspace) = document.query_selector(".main-workspace").unwrap() {
        workspace
            .parent_element()
            .unwrap()
            .insert_before(&panel, Some(&workspace))
            .unwrap();
    }

    wire_instrument_panel(document);
}

/// Deactivate the current tool-chain (clear the instrument panel).
pub fn deactivate_chain(document: &Document) {
    hide(document);
}

/// Get a human-readable label for a chain id.
fn chain_label(chain_id: &str) -> &str {
    match chain_id {
        "epistemic:modalities" => "Epistemic Modalities",
        "office:containers" => "Office Containers",
        "image:tools" => "Image Tools",
        "sheet:tools" => "Sheet Tools",
        "spatial:tools" => "Spatial Tools",
        "comm:containers" => "Communication Containers",
        "rights:tools" => "Rights Tools",
        "health:tools" => "Health Tools",
        "code:tools" => "Code Tools",
        "ai:tools" => "AI Tools",
        _ => chain_id,
    }
}

/// Get tools for a specific tool-chain id.
fn tools_for_chain(chain_id: &str) -> Vec<RibbonTool> {
    match chain_id {
        "epistemic:modalities" => vec![
            RibbonTool {
                id: "epi:objective",
                icon: "\u{1F52C}",
                label: "Objective",
                description: "Tag as objective",
            },
            RibbonTool {
                id: "epi:subjective",
                icon: "\u{1F9E0}",
                label: "Subjective",
                description: "Tag as subjective",
            },
            RibbonTool {
                id: "epi:inter",
                icon: "\u{1F91D}",
                label: "Intersubj.",
                description: "Tag as intersubjective",
            },
            RibbonTool {
                id: "epi:normative",
                icon: "\u{2696}",
                label: "Normative",
                description: "Tag as normative",
            },
        ],
        "office:containers" => vec![
            RibbonTool {
                id: "office:doc",
                icon: "\u{1F4C4}",
                label: "+ Doc",
                description: "Place document container",
            },
            RibbonTool {
                id: "office:ont",
                icon: "\u{1F4D6}",
                label: "+ Ontology",
                description: "Place ontology node",
            },
            RibbonTool {
                id: "office:slide",
                icon: "\u{1F4CA}",
                label: "+ Slide",
                description: "Place slide",
            },
        ],
        "image:tools" => vec![
            RibbonTool {
                id: "img:media",
                icon: "\u{1F3A8}",
                label: "+ Media",
                description: "Place media container",
            },
            RibbonTool {
                id: "img:marker",
                icon: "\u{1F4CD}",
                label: "Marker",
                description: "Draw marker",
            },
            RibbonTool {
                id: "img:heatmap",
                icon: "\u{1F525}",
                label: "Heatmap",
                description: "Spectral heatmap",
            },
        ],
        "sheet:tools" => vec![
            RibbonTool {
                id: "sheet:place",
                icon: "\u{1F4CA}",
                label: "+ Sheet",
                description: "Place sheet",
            },
            RibbonTool {
                id: "sheet:import",
                icon: "\u{21E9}",
                label: "Import",
                description: "Import CSV/CBOR",
            },
        ],
        "spatial:tools" => vec![
            RibbonTool {
                id: "spatial:map",
                icon: "\u{1F5FA}",
                label: "+ Map",
                description: "Place map",
            },
            RibbonTool {
                id: "spatial:3d",
                icon: "\u{1F3AF}",
                label: "+ 3D",
                description: "Place 3D viewport",
            },
            RibbonTool {
                id: "spatial:pin",
                icon: "\u{1F4CC}",
                label: "Pin",
                description: "Place pin",
            },
            RibbonTool {
                id: "spatial:track",
                icon: "\u{1F50D}",
                label: "Track",
                description: "Add track",
            },
        ],
        "comm:containers" => vec![
            RibbonTool {
                id: "comm:social",
                icon: "\u{1F4AC}",
                label: "+ Social",
                description: "Place social graph",
            },
            RibbonTool {
                id: "comm:webrtc",
                icon: "\u{1F4F7}",
                label: "+ WebRTC",
                description: "Place WebRTC",
            },
            RibbonTool {
                id: "comm:webview",
                icon: "\u{1F310}",
                label: "+ Webview",
                description: "Place webview",
            },
        ],
        "rights:tools" => vec![
            RibbonTool {
                id: "rights:group",
                icon: "\u{1F465}",
                label: "Authors",
                description: "Authors group",
            },
            RibbonTool {
                id: "rights:sign",
                icon: "\u{270D}",
                label: "Fiduciary",
                description: "Fiduciary sign",
            },
            RibbonTool {
                id: "rights:did",
                icon: "\u{1F194}",
                label: "DID",
                description: "DID sign",
            },
        ],
        "health:tools" => vec![
            RibbonTool {
                id: "health:place",
                icon: "\u{1FA7A}",
                label: "+ Health",
                description: "Place health container",
            },
            RibbonTool {
                id: "health:path",
                icon: "\u{1F52C}",
                label: "Pathology",
                description: "Pathology",
            },
            RibbonTool {
                id: "health:anat",
                icon: "\u{1F9B2}",
                label: "10D Anatomy",
                description: "10D anatomy",
            },
        ],
        "code:tools" => vec![
            RibbonTool {
                id: "code:vibe",
                icon: "\u{1F4BB}",
                label: "+ Vibe",
                description: "Place Vibe cell",
            },
            RibbonTool {
                id: "code:quin",
                icon: "\u{1F9EC}",
                label: "quin.statement",
                description: "Insert quin.statement",
            },
        ],
        "ai:tools" => vec![
            RibbonTool {
                id: "ai:coauthor",
                icon: "\u{1F9D1}",
                label: "Co-Author",
                description: "Co-author agent",
            },
            RibbonTool {
                id: "ai:extractor",
                icon: "\u{26CF}",
                label: "Extractor",
                description: "Extractor agent",
            },
            RibbonTool {
                id: "ai:sentinel",
                icon: "\u{1F6E1}",
                label: "Sentinel",
                description: "Sentinel guard",
            },
            RibbonTool {
                id: "ai:triad",
                icon: "\u{1F3A8}",
                label: "Triad",
                description: "Triad q42/p64/d10",
            },
        ],
        _ => vec![],
    }
}
