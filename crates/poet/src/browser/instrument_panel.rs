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
            show_tool_notification(&doc, &tool_id, &label);
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

fn show_tool_notification(document: &Document, tool_id: &str, label: &str) {
    // Remove existing notification
    if let Some(existing) = document.query_selector(".tool-notification").unwrap() {
        existing.remove();
    }

    let notif = document.create_element("div").unwrap();
    notif.set_class_name("tool-notification");
    let n_el: HtmlElement = notif.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "position: fixed; bottom: 40px; right: 16px; background: var(--surface-panel-elevated); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         padding: 10px 14px; font-size: 12px; color: var(--text-primary); \
         box-shadow: var(--shadow-lg); z-index: 500; max-width: 320px;",
    );
    notif.set_text_content(Some(&format!(
        "\u{1F4A1} {} ({}) \u{2014} present, engine wiring pending",
        label, tool_id
    )));
    if let Some(body) = document.body() {
        body.append_child(&notif).unwrap();
    }
    let notif_clone = notif.clone();
    let timeout = Closure::wrap(Box::new(move || {
        notif_clone.remove();
    }) as Box<dyn FnMut()>);
    super::interactions::set_timeout(timeout.as_ref().unchecked_ref(), 2500);
    timeout.forget();
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
                description: "Insert capability.invoke",
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
