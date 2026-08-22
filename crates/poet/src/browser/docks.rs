//! Dock rendering: toolbox sidebar, right dock (aura + pulse), bottom status bar.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use std::cell::RefCell;

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, Event, HtmlElement};

use crate::tool_chest::core::tool::ToolKind;
use crate::tool_chest::core::tool_chain::ToolChainMetadata;
use crate::tool_chest::core::toolbox::{Toolbox, ToolboxMetadata};

// ---------------------------------------------------------------------------
// Cloneable view models (the registry holds Box<dyn Tool> which is not Clone)
// ---------------------------------------------------------------------------

/// A cloneable view of a single tool's metadata for UI rendering.
#[derive(Clone, Debug)]
pub struct ToolView {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub kind: ToolKind,
    pub capability_scope: Option<String>,
    pub description: String,
}

/// A cloneable view of a tool-chain with its tools.
#[derive(Clone, Debug)]
pub struct ToolChainView {
    pub metadata: ToolChainMetadata,
    pub tools: Vec<ToolView>,
}

/// A cloneable view of a toolbox with its tool-chains.
#[derive(Clone, Debug)]
pub struct ToolboxView {
    pub metadata: ToolboxMetadata,
    pub chains: Vec<ToolChainView>,
}

/// Dock position orientations for 4-way docking architecture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockPosition {
    Left,
    Top,
    Right,
    Bottom,
}

impl DockPosition {
    pub fn as_str(&self) -> &'static str {
        match self {
            DockPosition::Left => "left",
            DockPosition::Top => "top",
            DockPosition::Right => "right",
            DockPosition::Bottom => "bottom",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "top" => DockPosition::Top,
            "right" => DockPosition::Right,
            "bottom" => DockPosition::Bottom,
            _ => DockPosition::Left,
        }
    }
}

/// Metadata for a toolbox family group.
#[derive(Clone, Debug)]
pub struct ToolboxFamily {
    pub id: String,
    pub label: String,
    pub icon: String,
}

/// Get the ordered list of 12 master toolbox families.
pub fn family_order() -> Vec<ToolboxFamily> {
    vec![
        ToolboxFamily {
            id: "epistemic".into(),
            label: "Epistemic Mindware".into(),
            icon: "\u{1F9ED}".into(), // 🧭
        },
        ToolboxFamily {
            id: "authoring".into(),
            label: "Word Processor & CML".into(),
            icon: "\u{1F4DD}".into(), // 📝
        },
        ToolboxFamily {
            id: "sheet".into(),
            label: "Spreadsheets & Tensors".into(),
            icon: "\u{1F4CA}".into(), // 📊
        },
        ToolboxFamily {
            id: "graphics".into(),
            label: "Graphics & Vector".into(),
            icon: "\u{1F3A8}".into(), // 🎨
        },
        ToolboxFamily {
            id: "spatial".into(),
            label: "3D & Geospatial".into(),
            icon: "\u{1F9CA}".into(), // 🧊
        },
        ToolboxFamily {
            id: "audio".into(),
            label: "Triad Formant Audio".into(),
            icon: "\u{1F399}\u{FE0F}".into(), // 🎙️
        },
        ToolboxFamily {
            id: "code".into(),
            label: "Code IDE & Vibe REPL".into(),
            icon: "\u{1F4BB}".into(), // 💻
        },
        ToolboxFamily {
            id: "erp".into(),
            label: "Cooperative ERP & PM".into(),
            icon: "\u{1F4C5}".into(), // 📅
        },
        ToolboxFamily {
            id: "mail".into(),
            label: "Mail & Web Presence".into(),
            icon: "\u{2709}\u{FE0F}".into(), // ✉️
        },
        ToolboxFamily {
            id: "lab".into(),
            label: "Scientific & Clinical".into(),
            icon: "\u{1F52C}".into(), // 🔬
        },
        ToolboxFamily {
            id: "ai".into(),
            label: "AI Co-Pilot & Sentinel".into(),
            icon: "\u{2728}".into(), // ✨
        },
        ToolboxFamily {
            id: "governance".into(),
            label: "Governance & Rights".into(),
            icon: "\u{2696}\u{FE0F}".into(), // ⚖️
        },
        ToolboxFamily {
            id: "sdn".into(),
            label: "SDN & Economics".into(),
            icon: "\u{1F310}".into(), // 🌐
        },
    ]
}

/// Extract cloneable views from the registry's toolboxes.
pub fn extract_toolbox_views(toolboxes: &[Toolbox]) -> Vec<ToolboxView> {
    toolboxes
        .iter()
        .map(|tb| ToolboxView {
            metadata: tb.metadata().clone(),
            chains: tb
                .chains()
                .iter()
                .map(|chain| ToolChainView {
                    metadata: chain.metadata().clone(),
                    tools: chain
                        .tools()
                        .iter()
                        .map(|tool| {
                            let m = tool.metadata();
                            ToolView {
                                id: m.id.clone(),
                                label: m.label.clone(),
                                icon: m.icon.clone(),
                                kind: m.kind,
                                capability_scope: m.capability_scope.clone(),
                                description: m.description.clone(),
                            }
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Thread-local storage for flyout rendering
// ---------------------------------------------------------------------------

thread_local! {
    static TOOLBOX_VIEWS: RefCell<Vec<ToolboxView>> = RefCell::new(Vec::new());
}

/// Store toolbox views in the thread-local for access from click handlers.
pub fn store_toolbox_views(views: Vec<ToolboxView>) {
    TOOLBOX_VIEWS.with(|v| {
        *v.borrow_mut() = views;
    });
}

// ---------------------------------------------------------------------------
// Glyph mapping
// ---------------------------------------------------------------------------

/// Map a toolbox id to a display glyph covering all 12 themed Master Toolboxes.
fn toolbox_glyph(id: &str) -> &'static str {
    match id {
        "epistemic" => "\u{1F9ED}",             // 🧭
        "office" | "word_processor" | "tb_word_processor" | "doc" => "\u{1F4DD}", // 📝
        "sheet" | "tb_spreadsheet" => "\u{1F4CA}", // 📊
        "image" | "graphics" | "tb_graphics" => "\u{1F3A8}", // 🎨
        "spatial" | "3d" | "tb_3d_spatial" => "\u{1F9CA}", // 🧊
        "audio" | "audio_synth" | "tb_audio_synth" => "\u{1F399}\u{FE0F}", // 🎙️
        "code" | "tb_code_ide" => "\u{1F4BB}",    // 💻
        "communication" | "mail" | "tb_mail_publish" => "\u{2709}\u{FE0F}", // ✉️
        "erp" | "tb_erp_workstream" => "\u{1F4C5}", // 📅
        "lab" | "science" | "tb_scientific_lab" => "\u{1F52C}", // 🔬
        "ai" | "tb_ai_copilot" => "\u{2728}",     // ✨
        "rights" | "governance" | "tb_governance_rights" => "\u{2696}\u{FE0F}", // ⚖️
        "sdn" | "tb_sdn_cooperative" => "\u{1F310}", // 🌐
        "health" => "\u{1FA7A}",                  // 🩺
        _ => "\u{1F9E9}",
    }
}

/// Map a tool icon identifier to a display glyph.
fn tool_glyph(icon: &str) -> &'static str {
    match icon {
        "doc" => "\u{1F4C4}",
        "ontology" => "\u{1F4D6}",
        "slide" => "\u{1F4CA}",
        "media" => "\u{1F3A8}",
        "marker" => "\u{1F4CD}",
        "heatmap" => "\u{1F525}",
        "sheet" => "\u{1F4CA}",
        "import" => "\u{21E9}",
        "map" => "\u{1F5FA}",
        "3d" => "\u{1F3AF}",
        "pin" => "\u{1F4CC}",
        "track" => "\u{1F50D}",
        "social" => "\u{1F4AC}",
        "webrtc" => "\u{1F4F7}",
        "webview" => "\u{1F310}",
        "group" => "\u{1F465}",
        "sign" => "\u{270D}",
        "did" => "\u{1F194}",
        "health" => "\u{1FA7A}",
        "pathology" => "\u{1F52C}",
        "anatomy" => "\u{1F9B2}",
        "vibe" => "\u{1F4BB}",
        "quin" => "\u{1F9EC}",
        "coauthor" => "\u{1F9D1}",
        "extractor" => "\u{26CF}",
        "sentinel" => "\u{1F6E1}",
        "triad" => "\u{1F3A8}",
        "objective" => "\u{1F4CD}",
        "subjective" => "\u{1F9ED}",
        "intersubjective" => "\u{1F91D}",
        "normative" => "\u{2696}",
        _ => "\u{1F4A1}",
    }
}

/// Short kind label for the tool button badge.
fn kind_label(kind: ToolKind) -> &'static str {
    match kind {
        ToolKind::PlaceContainer => "place",
        ToolKind::RunAction => "action",
        ToolKind::Query => "query",
        ToolKind::Navigate => "nav",
        ToolKind::Toggle => "toggle",
    }
}

// ---------------------------------------------------------------------------
// Dock builder with 4-Way Docking Architecture
// ---------------------------------------------------------------------------

/// Build the toolbox dock from a populated registry with 4-way docking anchor controls.
pub fn build_toolbox_dock(document: &Document, toolboxes: &[Toolbox]) -> Element {
    let dock = document.create_element("div").unwrap();
    dock.set_class_name("toolbox-dock dock-pos-left");

    // Dock Header: Brand + 4-Way Docking Anchor Bar
    let dock_header = document.create_element("div").unwrap();
    dock_header.set_class_name("dock-master-header");
    let dh_el: HtmlElement = dock_header.clone().dyn_into().unwrap();
    dh_el.style().set_css_text(
        "display: flex; align-items: center; justify-content: space-between; \
         padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); margin-bottom: 4px;",
    );

    let title_span = document.create_element("span").unwrap();
    let ts_el: HtmlElement = title_span.clone().dyn_into().unwrap();
    ts_el.style().set_css_text(
        "font-size: 9px; font-weight: 700; color: var(--accent-cyan); \
         text-transform: uppercase; letter-spacing: 0.5px; font-family: var(--font-mono);",
    );
    title_span.set_text_content(Some("\u{1F9F0} Tool Chest"));
    dock_header.append_child(&title_span).unwrap();

    // 4-Way Dock Anchor Controls
    let anchor_bar = document.create_element("div").unwrap();
    anchor_bar.set_class_name("dock-anchor-bar");
    let ab_el: HtmlElement = anchor_bar.clone().dyn_into().unwrap();
    ab_el.style().set_css_text("display: flex; gap: 2px;");

    let positions = [
        ("left", "\u{25C0}"),
        ("top", "\u{25B2}"),
        ("right", "\u{25B6}"),
        ("bottom", "\u{25BC}"),
    ];

    for (pos_id, glyph) in &positions {
        let pos_btn = document.create_element("button").unwrap();
        pos_btn.set_class_name("dock-pos-btn");
        pos_btn.set_attribute("data-pos", pos_id).unwrap();
        pos_btn.set_attribute("title", &format!("Dock {}", pos_id)).unwrap();
        let pb_el: HtmlElement = pos_btn.clone().dyn_into().unwrap();
        pb_el.style().set_css_text(
            "padding: 1px 3px; font-size: 8px; background: transparent; border: 1px solid transparent; \
             border-radius: 2px; color: var(--text-muted); cursor: pointer; transition: var(--trans-fast);",
        );
        pos_btn.set_text_content(Some(glyph));

        let dock_clone = dock.clone();
        let pos_str = pos_id.to_string();
        let pos_closure = Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            let d_el: HtmlElement = dock_clone.clone().dyn_into().unwrap();
            d_el.set_class_name(&format!("toolbox-dock dock-pos-{}", pos_str));
            if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok()).flatten() {
                let _ = storage.set_item("qualia_dock_pos", &pos_str);
            }
        }) as Box<dyn FnMut(web_sys::MouseEvent)>);
        pos_btn.add_event_listener_with_callback("click", pos_closure.as_ref().unchecked_ref()).unwrap();
        pos_closure.forget();

        anchor_bar.append_child(&pos_btn).unwrap();
    }
    dock_header.append_child(&anchor_bar).unwrap();
    dock.append_child(&dock_header).unwrap();

    let families = family_order();
    let mut first_toolbox = true;

    for family in &families {
        // Find toolboxes in this family
        let family_toolboxes: Vec<&Toolbox> = toolboxes
            .iter()
            .filter(|tb| tb.metadata().family == family.id)
            .collect();

        if family_toolboxes.is_empty() {
            continue;
        }

        // Family section
        let section = document.create_element("div").unwrap();
        section.set_class_name("dock-family-section");
        section.set_attribute("data-family", &family.id).unwrap();

        // Family header (collapsible)
        let header = document.create_element("button").unwrap();
        header.set_class_name("dock-family-header");
        header.set_attribute("data-family", &family.id).unwrap();
        header.set_attribute("title", &family.label).unwrap();

        let family_icon = document.create_element("span").unwrap();
        family_icon.set_class_name("dock-family-icon");
        family_icon.set_text_content(Some(&family.icon));
        header.append_child(&family_icon).unwrap();

        let family_label = document.create_element("span").unwrap();
        family_label.set_class_name("dock-family-label");
        family_label.set_text_content(Some(&family.label));
        header.append_child(&family_label).unwrap();

        let chevron = document.create_element("span").unwrap();
        chevron.set_class_name("dock-family-chevron");
        chevron.set_text_content(Some("\u{25BE}"));
        header.append_child(&chevron).unwrap();

        section.append_child(&header).unwrap();

        // Toolbox buttons (children, shown by default for first family)
        let children = document.create_element("div").unwrap();
        children.set_class_name("dock-family-children");
        if first_toolbox {
            children.class_list().add_1("expanded").unwrap();
        }

        for toolbox in &family_toolboxes {
            let meta = toolbox.metadata();
            let btn = document.create_element("button").unwrap();
            btn.set_class_name("toolbox-dock-btn");
            if first_toolbox {
                btn.class_list().add_1("active").unwrap();
                first_toolbox = false;
            }
            btn.set_attribute("data-toolbox", &meta.id).unwrap();
            btn.set_text_content(Some(toolbox_glyph(&meta.id)));

            let tooltip = document.create_element("span").unwrap();
            tooltip.set_class_name("dock-tooltip");
            tooltip.set_text_content(Some(&meta.label));
            btn.append_child(&tooltip).unwrap();

            children.append_child(&btn).unwrap();
        }

        section.append_child(&children).unwrap();
        dock.append_child(&section).unwrap();
    }

    dock
}

// ---------------------------------------------------------------------------
// Flyout panel — shows tool-chains and tools for the active toolbox
// ---------------------------------------------------------------------------

/// Show or replace the flyout panel for the given toolbox id.
/// Removes any existing flyout first.
pub fn show_flyout(document: &Document, toolbox_id: &str) {
    // Remove existing flyout
    if let Some(existing) = document.query_selector(".toolbox-flyout").unwrap() {
        existing.remove();
    }

    let view = TOOLBOX_VIEWS.with(|v| {
        v.borrow()
            .iter()
            .find(|t| t.metadata.id == toolbox_id)
            .cloned()
    });

    let view = match view {
        Some(v) => v,
        None => return,
    };

    let flyout = document.create_element("div").unwrap();
    flyout.set_class_name("toolbox-flyout");

    // Header
    let header = document.create_element("div").unwrap();
    header.set_class_name("toolbox-flyout-header");
    header.set_text_content(Some(&view.metadata.label));
    flyout.append_child(&header).unwrap();

    // Tool-chains
    for chain in &view.chains {
        let group = document.create_element("div").unwrap();
        group.set_class_name("toolchain-group");

        let chain_label = document.create_element("div").unwrap();
        chain_label.set_class_name("toolchain-label");
        chain_label
            .set_attribute("data-chain-id", &chain.metadata.id)
            .unwrap();
        chain_label
            .set_attribute("data-toolbox-id", &view.metadata.id)
            .unwrap();
        chain_label.set_attribute("draggable", "true").unwrap();
        chain_label
            .set_attribute(
                "title",
                "Click to activate on focused surface, or drag onto a container",
            )
            .unwrap();

        let chain_icon = document.create_element("span").unwrap();
        chain_icon.set_class_name("toolchain-label-icon");
        chain_icon.set_text_content(Some("\u{2630}"));
        chain_label.append_child(&chain_icon).unwrap();

        let chain_text = document.create_element("span").unwrap();
        chain_text.set_class_name("toolchain-label-text");
        chain_text.set_text_content(Some(&chain.metadata.label));
        chain_label.append_child(&chain_text).unwrap();

        group.append_child(&chain_label).unwrap();

        for tool in &chain.tools {
            let btn = document.create_element("button").unwrap();
            btn.set_class_name("tool-btn");
            btn.set_attribute("data-tool-id", &tool.id).unwrap();
            btn.set_attribute("data-chain-id", &chain.metadata.id)
                .unwrap();
            btn.set_attribute("title", &tool.description).unwrap();

            let icon_el = document.create_element("span").unwrap();
            icon_el.set_class_name("tool-btn-icon");
            icon_el.set_text_content(Some(tool_glyph(&tool.icon)));
            btn.append_child(&icon_el).unwrap();

            let label_el = document.create_element("span").unwrap();
            label_el.set_class_name("tool-btn-label");
            label_el.set_text_content(Some(&tool.label));
            btn.append_child(&label_el).unwrap();

            let kind_el = document.create_element("span").unwrap();
            kind_el.set_class_name("tool-btn-kind");
            kind_el.set_text_content(Some(kind_label(tool.kind)));
            btn.append_child(&kind_el).unwrap();

            group.append_child(&btn).unwrap();
        }

        flyout.append_child(&group).unwrap();
    }

    // Append to the workspace (so it positions relative to the dock)
    if let Some(workspace) = document.query_selector(".main-workspace").unwrap() {
        workspace.append_child(&flyout).unwrap();
    } else if let Some(body) = document.body() {
        body.append_child(&flyout).unwrap();
    }
}

/// Hide the flyout panel.
pub fn hide_flyout(document: &Document) {
    if let Some(existing) = document.query_selector(".toolbox-flyout").unwrap() {
        existing.remove();
    }
}

/// Build the right dock (aura tray + pulse stream).
pub fn build_right_dock(document: &Document) -> Element {
    let dock = document.create_element("div").unwrap();
    dock.set_class_name("right-dock");
    dock.set_id("right-dock");

    // Collapse toggle button (shown when dock is collapsed)
    let expand_btn = document.create_element("button").unwrap();
    expand_btn.set_class_name("right-dock-expand-btn");
    expand_btn.set_id("right-dock-expand-btn");
    let eb_el: HtmlElement = expand_btn.clone().dyn_into().unwrap();
    eb_el.style().set_css_text(
        "display: none; position: absolute; right: 0; top: 50%; \
         transform: translateY(-50%); width: 20px; height: 60px; \
         background: var(--surface-panel); border: 1px solid var(--border-subtle); \
         border-right: none; border-radius: var(--radius-xs) 0 0 var(--radius-xs); \
         color: var(--text-muted); cursor: pointer; font-size: 14px; \
         z-index: 100; writing-mode: vertical-rl; padding: 4px;",
    );
    expand_btn.set_text_content(Some("\u{25C0} Dock"));
    dock.append_child(&expand_btn).unwrap();

    // Dock content wrapper (hidden when collapsed)
    let content = document.create_element("div").unwrap();
    content.set_class_name("right-dock-content");
    content.set_id("right-dock-content");

    // Collapse button (shown when dock is expanded)
    let collapse_btn = document.create_element("button").unwrap();
    collapse_btn.set_class_name("right-dock-collapse-btn");
    let cb_el: HtmlElement = collapse_btn.clone().dyn_into().unwrap();
    cb_el.style().set_css_text(
        "position: absolute; right: 4px; top: 4px; width: 18px; height: 18px; \
         background: transparent; border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); color: var(--text-muted); \
         cursor: pointer; font-size: 10px; z-index: 10; \
         display: flex; align-items: center; justify-content: center;",
    );
    collapse_btn.set_text_content(Some("\u{25B6}"));
    content.append_child(&collapse_btn).unwrap();

    // Aura Tray — wired to diagnostics module
    let aura = document.create_element("div").unwrap();
    aura.set_class_name("dock-panel");
    let aura_header = document.create_element("div").unwrap();
    aura_header.set_class_name("dock-panel-header");
    aura_header.set_text_content(Some("Aura Tray"));
    aura.append_child(&aura_header).unwrap();

    let shacl_results = super::diagnostics::default_shacl_results();
    let aura_body = super::diagnostics::render_aura_tray(document, &shacl_results);
    aura.append_child(&aura_body).unwrap();
    content.append_child(&aura).unwrap();

    // Pulse Stream — wired to diagnostics module
    let pulse = document.create_element("div").unwrap();
    pulse.set_class_name("dock-panel");
    let pulse_el: HtmlElement = pulse.clone().dyn_into().unwrap();
    pulse_el
        .style()
        .set_css_text("flex: 1; overflow: hidden; display: flex; flex-direction: column;");

    let pulse_header = document.create_element("div").unwrap();
    pulse_header.set_class_name("dock-panel-header");
    pulse_header.set_text_content(Some("Pulse Stream"));
    pulse.append_child(&pulse_header).unwrap();

    let pulse_events = super::diagnostics::default_pulse_events();
    let pulse_body = super::diagnostics::render_pulse_stream(document, &pulse_events);
    pulse.append_child(&pulse_body).unwrap();
    content.append_child(&pulse).unwrap();

    // Job Center — background job queue
    let jobs = super::diagnostics::default_jobs();
    let job_panel = super::diagnostics::render_job_center(document, &jobs);
    content.append_child(&job_panel).unwrap();

    dock.append_child(&content).unwrap();

    // Wire collapse/expand
    let content_clone = content.clone();
    let dock_clone = dock.clone();
    let expand_btn_clone1 = expand_btn.clone();
    let expand_btn_clone2 = expand_btn.clone();

    let collapse_closure = Closure::wrap(Box::new(move |_e: Event| {
        let content_el: HtmlElement = content_clone.clone().dyn_into().unwrap();
        content_el.style().set_property("display", "none").unwrap();
        let eb: HtmlElement = expand_btn_clone1.clone().dyn_into().unwrap();
        eb.style().set_property("display", "flex").unwrap();
        let d_el: HtmlElement = dock_clone.clone().dyn_into().unwrap();
        d_el.style().set_property("width", "20px").unwrap();
        d_el.style().set_property("min-width", "20px").unwrap();
    }) as Box<dyn FnMut(Event)>);
    collapse_btn
        .add_event_listener_with_callback("click", collapse_closure.as_ref().unchecked_ref())
        .unwrap();
    collapse_closure.forget();

    let content_clone2 = content.clone();
    let dock_clone2 = dock.clone();
    let expand_closure = Closure::wrap(Box::new(move |_e: Event| {
        let content_el: HtmlElement = content_clone2.clone().dyn_into().unwrap();
        content_el.style().set_property("display", "").unwrap();
        let eb: HtmlElement = expand_btn_clone2.clone().dyn_into().unwrap();
        eb.style().set_property("display", "none").unwrap();
        let d_el: HtmlElement = dock_clone2.clone().dyn_into().unwrap();
        d_el.style().set_property("width", "").unwrap();
        d_el.style().set_property("min-width", "").unwrap();
    }) as Box<dyn FnMut(Event)>);
    expand_btn
        .add_event_listener_with_callback("click", expand_closure.as_ref().unchecked_ref())
        .unwrap();
    expand_closure.forget();

    dock
}

/// Build the bottom status bar.
pub fn build_bottom_statusbar(document: &Document) -> Element {
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("bottom-statusbar");

    // Left section
    let left = document.create_element("div").unwrap();
    left.set_class_name("statusbar-section");

    let graph = document.create_element("div").unwrap();
    graph.set_class_name("statusbar-item");
    let g_label = document.create_element("span").unwrap();
    g_label.set_class_name("statusbar-label");
    g_label.set_text_content(Some("Graph:"));
    let g_val = document.create_element("span").unwrap();
    g_val.set_class_name("statusbar-value");
    g_val.set_text_content(Some("catchment_sites"));
    graph.append_child(&g_label).unwrap();
    graph.append_child(&g_val).unwrap();
    left.append_child(&graph).unwrap();

    let merkle = document.create_element("div").unwrap();
    merkle.set_class_name("statusbar-item");
    let m_label = document.create_element("span").unwrap();
    m_label.set_class_name("statusbar-label");
    m_label.set_text_content(Some("Merkle:"));
    let m_val = document.create_element("span").unwrap();
    m_val.set_class_name("statusbar-value");
    m_val.set_text_content(Some("0x8f...a42"));
    merkle.append_child(&m_label).unwrap();
    merkle.append_child(&m_val).unwrap();
    left.append_child(&merkle).unwrap();

    bar.append_child(&left).unwrap();

    // Right section
    let right = document.create_element("div").unwrap();
    right.set_class_name("statusbar-section");

    let gas = document.create_element("div").unwrap();
    gas.set_class_name("statusbar-item");
    let g_label = document.create_element("span").unwrap();
    g_label.set_class_name("statusbar-label");
    g_label.set_text_content(Some("Gas:"));
    let g_val = document.create_element("span").unwrap();
    g_val.set_class_name("statusbar-gas");
    g_val.set_text_content(Some("984,500 / 1M"));
    gas.append_child(&g_label).unwrap();
    gas.append_child(&g_val).unwrap();
    right.append_child(&gas).unwrap();

    let strata = document.create_element("div").unwrap();
    strata.set_class_name("statusbar-item");
    let s_label = document.create_element("span").unwrap();
    s_label.set_class_name("statusbar-label");
    s_label.set_text_content(Some("Strata:"));
    let s_val = document.create_element("span").unwrap();
    s_val.set_class_name("statusbar-value");
    s_val.set_text_content(Some("social, legal"));
    strata.append_child(&s_label).unwrap();
    strata.append_child(&s_val).unwrap();
    right.append_child(&strata).unwrap();

    bar.append_child(&right).unwrap();
    bar
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_toolbox_families_count() {
        let families = family_order();
        assert!(families.len() >= 12, "Expected at least 12 master toolbox families, got {}", families.len());
        let ids: Vec<&str> = families.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"epistemic"));
        assert!(ids.contains(&"authoring"));
        assert!(ids.contains(&"sheet"));
        assert!(ids.contains(&"graphics"));
        assert!(ids.contains(&"spatial"));
        assert!(ids.contains(&"audio"));
        assert!(ids.contains(&"code"));
        assert!(ids.contains(&"erp"));
        assert!(ids.contains(&"mail"));
        assert!(ids.contains(&"lab"));
        assert!(ids.contains(&"ai"));
        assert!(ids.contains(&"governance"));
        assert!(ids.contains(&"sdn"));
    }

    #[test]
    fn test_dock_position_conversions() {
        assert_eq!(DockPosition::from_str("top"), DockPosition::Top);
        assert_eq!(DockPosition::from_str("right"), DockPosition::Right);
        assert_eq!(DockPosition::from_str("bottom"), DockPosition::Bottom);
        assert_eq!(DockPosition::from_str("left"), DockPosition::Left);
        assert_eq!(DockPosition::from_str("invalid"), DockPosition::Left);

        assert_eq!(DockPosition::Top.as_str(), "top");
        assert_eq!(DockPosition::Right.as_str(), "right");
        assert_eq!(DockPosition::Bottom.as_str(), "bottom");
        assert_eq!(DockPosition::Left.as_str(), "left");
    }
}
