use dioxus::prelude::*;
use qualia_core_db::NQuin;
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::{EventSource, MessageEvent};

use crate::pane_registry::{
    builtin_pane_definitions, category_label, find_pane, PaneCategory, PaneDefinition,
};
use crate::theme_engine::{
    builtin_theme_catalog, collect_stylesheets, join_theme_classes, render_scope_tokens,
    resolve_theme, ResolvedTheme, ThemeBinding, ThemeDefinition,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum LayoutStrategy {
    PointGrid {
        width_points: u16,
        height_points: u16,
        snap_step: u16,
        gutter: u16,
    },
    CssGrid { cols: u8, rows: u8, gap: u8 },
    FlexBox,
    Masonry,
}

impl Default for LayoutStrategy {
    fn default() -> Self {
        Self::PointGrid {
            width_points: 96,
            height_points: 64,
            snap_step: 2,
            gutter: 2,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PresentationMode {
    GridBound,
    NodeRelational,
    Spatial,
}

impl Default for PresentationMode {
    fn default() -> Self {
        Self::GridBound
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CoordinateSpace {
    GlobalCartesian,
    RelativeAnchored,
}

impl Default for CoordinateSpace {
    fn default() -> Self {
        Self::GlobalCartesian
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum UiMode {
    NativeDioxus,
    IFrameSandbox,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum LayerBehavior {
    Docked,
    FloatingOverlay,
    ModalOverlay,
    FullCanvas,
}

impl Default for LayerBehavior {
    fn default() -> Self {
        Self::Docked
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PanePlacement {
    pub component_id: String,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    pub data_bindings: Vec<String>,
    #[serde(default)]
    pub binds_rpc: Option<String>,
    #[serde(default)]
    pub requires_capability: Vec<String>,
    #[serde(default)]
    pub ui_mode: Option<UiMode>,
    #[serde(default)]
    pub layer: LayerBehavior,
    #[serde(default)]
    pub anchor: Option<String>,
    #[serde(default)]
    pub min_w_points: u16,
    #[serde(default)]
    pub min_h_points: u16,
    #[serde(default)]
    pub supported_presentations: Vec<PresentationMode>,
    #[serde(default)]
    pub theme: ThemeBinding,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Page {
    pub url_path: String,
    pub name: String,
    #[serde(default)]
    pub layout_strategy: LayoutStrategy,
    pub panes: Vec<PanePlacement>,
    #[serde(default)]
    pub presentation_mode: PresentationMode,
    #[serde(default)]
    pub coordinate_space: CoordinateSpace,
    #[serde(default)]
    pub pan_and_zoom: bool,
    #[serde(default)]
    pub theme: ThemeBinding,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct WebizenWorkspace {
    pub pages: Vec<Page>,
    #[serde(default)]
    pub theme_tokens: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub themes: Vec<ThemeDefinition>,
    #[serde(default)]
    pub environment_theme: ThemeBinding,
    #[serde(default)]
    pub app_theme: ThemeBinding,
}

// ─────────────────────────────────────────────────────────────
// The main DynamicPage component
// ─────────────────────────────────────────────────────────────

#[component]
pub fn DynamicPage(path: Vec<String>) -> Element {
    let mock_quin = use_signal(|| NQuin {
        subject: 0,
        predicate: 0,
        object: 0,
        context: 0,
        metadata: 0,
        parity: 0,
    });

    let mut workspace = use_signal(WebizenWorkspace::default);
    let selected_pane_index = use_signal(|| None::<usize>);
    let pane_palette = use_signal(builtin_pane_definitions);

    // ── Boot Rehydration ───────────────────────────────────
    use_effect(move || {
        spawn(async move {
            // Use default if nothing loads
            if workspace.read().pages.is_empty() {
                if let Ok(res) = reqwest::get("http://127.0.0.1:8080/manifest").await {
                    if let Ok(data) = res.json::<WebizenWorkspace>().await {
                        workspace.set(data);
                    }          
                }
            }
        });
    });

    // ── Native Handshake Probe ─────────────────────────────
    let mut is_native_llm_active = use_signal(|| false);

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if let Ok(ws) = web_sys::WebSocket::new("ws://127.0.0.1:4242") {
            let onopen = Closure::wrap(Box::new(move |_e: web_sys::Event| {
                is_native_llm_active.set(true);
            }) as Box<dyn FnMut(web_sys::Event)>);
            ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
            onopen.forget();
        }
    });

    #[cfg(not(target_arch = "wasm32"))]
    use_effect(move || {
        is_native_llm_active.set(true); // Always true on native build
    });

    // 🔴🔴 Telemetry SSE 🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴
    let mut telemetry_logs = use_signal(Vec::<String>::new);

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if let Ok(es) = EventSource::new("http://127.0.0.1:8080/telemetry") {
            let callback = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Some(txt) = e.data().as_string() {
                    telemetry_logs.write().push(txt.clone());
                    if telemetry_logs.read().len() > 10 {
                        telemetry_logs.write().remove(0);
                    }
                }
            }) as Box<dyn FnMut(MessageEvent)>);

            es.set_onmessage(Some(callback.as_ref().unchecked_ref()));
            callback.forget();
        }
    });

    #[cfg(not(target_arch = "wasm32"))]
    use_effect(move || {
        // Desktop natively polling or disabled for now to avoid web_sys panic
    });

    // ── Drag-and-Drop: handle drop on the canvas ───────────
    let on_canvas_drop = {
        let mut workspace = workspace.clone();
        move |evt: Event<DragData>| {
            evt.prevent_default();
            // Retrieve the component_id that was set in drag_start
            let dt = evt.data().data_transfer();
            if let Some(component_id) = dt.get_data("application/x-qualia-pane-id") {
                if !component_id.is_empty() {
                    // Look up default dimensions from the registry
                    let (default_w, default_h) = find_pane(&component_id)
                        .map(|p| (p.default_w, p.default_h))
                        .unwrap_or((4, 2));

                    let new_pane = PanePlacement {
                        component_id: component_id.clone(),
                        x: 4,
                        y: 4 + (workspace.read().pages.first().map(|p| p.panes.len()).unwrap_or(0) as u16 * 6),
                        w: (default_w as u16).saturating_mul(6),
                        h: (default_h as u16).saturating_mul(6),
                        data_bindings: vec![],
                        binds_rpc: if component_id == "custom-web-module" { Some("ws://127.0.0.1:9001".into()) } else { None },
                        requires_capability: vec![],
                        ui_mode: if component_id == "custom-web-module" { Some(UiMode::IFrameSandbox) } else { None },
                        layer: if component_id == "custom-web-module" { LayerBehavior::FloatingOverlay } else { LayerBehavior::Docked },
                        anchor: None,
                        min_w_points: (default_w as u16).saturating_mul(4),
                        min_h_points: (default_h as u16).saturating_mul(4),
                        supported_presentations: vec![PresentationMode::GridBound],
                        theme: ThemeBinding::default(),
                    };

                    let mut ws = workspace.write();
                    if let Some(page) = ws.pages.first_mut() {
                        page.panes.push(new_pane);
                    }
                }
            }
        }
    };

    let on_canvas_dragover = |evt: Event<DragData>| {
        evt.prevent_default();
    };

    // ── Deploy handler ─────────────────────────────────────
    let deploy_workspace = {
        let workspace = workspace.clone();
        move |_| {
            spawn(async move {
                let current_workspace = workspace.read().clone();
                let payload = serde_json::to_string(&current_workspace).unwrap_or_default();

                let client = reqwest::Client::new();
                let _ = client
                    .post("http://127.0.0.1:8080/manifest")
                    .header("Content-Type", "application/yaml-ld-q42")
                    .body(payload)
                    .send()
                    .await;
            });
        }
    };

    // ── Delete selected pane ───────────────────────────────
    let delete_selected_pane = {
        let mut workspace = workspace.clone();
        let mut selected_pane_index = selected_pane_index.clone();
        move |_| {
            let idx_opt = *selected_pane_index.read();
            if let Some(idx) = idx_opt {
                let mut ws = workspace.write();
                if let Some(page) = ws.pages.first_mut() {
                    if idx < page.panes.len() {
                        page.panes.remove(idx);
                    }
                }
                selected_pane_index.set(None);
            }
        }
    };

    // ── Routing ────────────────────────────────────────────
    let current_path = format!("/{}", path.join("/"));
    let ws = workspace.read();
    let current_page = ws.pages.iter().find(|p| p.url_path == current_path).cloned();
    let theme_catalog = if ws.themes.is_empty() {
        builtin_theme_catalog()
    } else {
        ws.themes.clone()
    };
    let mut environment_theme = resolve_theme(Some(&ws.environment_theme), &theme_catalog);
    environment_theme.tokens.extend(ws.theme_tokens.clone());
    let app_theme = resolve_theme(Some(&ws.app_theme), &theme_catalog);
    let page_theme = current_page
        .as_ref()
        .map(|page| resolve_theme(Some(&page.theme), &theme_catalog))
        .unwrap_or_default();
    let pane_themes = current_page
        .as_ref()
        .map(|page| {
            page.panes
                .iter()
                .map(|pane| resolve_theme(Some(&pane.theme), &theme_catalog))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let all_themes = std::iter::once(&environment_theme)
        .chain(std::iter::once(&app_theme))
        .chain(std::iter::once(&page_theme))
        .chain(pane_themes.iter())
        .collect::<Vec<_>>();
    let theme_stylesheets = collect_stylesheets(&all_themes);
    let mut theme_css = String::new();
    if let Some(css) = render_scope_tokens(":root", &environment_theme) {
        theme_css.push_str(&css);
    }
    if let Some(css) = render_scope_tokens(".webizen-studio-shell", &app_theme) {
        theme_css.push_str(&css);
    }
    if let Some(css) = render_scope_tokens(".webizen-page-shell", &page_theme) {
        theme_css.push_str(&css);
    }
    for (idx, pane_theme) in pane_themes.iter().enumerate() {
        if let Some(css) = render_scope_tokens(
            &format!(".webizen-module-pane[data-pane-index='{}']", idx),
            pane_theme,
        ) {
            theme_css.push_str(&css);
        }
    }

    // ── Create New Page ────────────────────────────────────
    let create_new_page = {
        let mut workspace = workspace.clone();
        move |_| {
            let mut ws = workspace.write();
            let p_len = ws.pages.len() + 1;
            ws.pages.push(Page {
                url_path: format!("/page-{}", p_len),
                name: format!("New Page {}", p_len),
                layout_strategy: LayoutStrategy::default(),
                panes: vec![],
                presentation_mode: PresentationMode::GridBound,
                coordinate_space: CoordinateSpace::GlobalCartesian,
                pan_and_zoom: true,
                theme: ThemeBinding::default(),
            });
        }
    };

    rsx! {
        for href in theme_stylesheets.iter() {
            document::Link { rel: "stylesheet", href: "{href}" }
        }

        style { "{theme_css}" }

        div {
            class: "{join_theme_classes(\"webizen-studio-shell\", &app_theme)}",
            "data-theme-scope": "app",
            "data-theme": "{app_theme.theme_key.clone().unwrap_or_default()}",
            style: "flex: 1; display: grid; grid-template-columns: 240px 1fr 280px; gap: 0; height: calc(100vh - 60px);",

            // ════════════════════════════════════════════════
            // LEFT SIDEBAR: Pages + Component Palette
            // ════════════════════════════════════════════════
            div {
                style: "background: var(--qualia-surface, #111); border-right: 1px solid var(--qualia-border, #333); padding: 1rem; overflow-y: auto; display: flex; flex-direction: column; gap: 0.75rem;",

                // Page Navigation
                div {
                    style: "margin-bottom: 0.5rem;",
                    div {
                        style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem;",
                        h3 {
                            style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0;",
                            "Pages"
                        }
                        button {
                            style: "background: transparent; border: 1px solid var(--qualia-border, #444); color: var(--qualia-accent, #0ff); padding: 0.2rem 0.4rem; border-radius: 4px; font-size: 0.65rem; cursor: pointer;",
                            onclick: create_new_page,
                            "+ New"
                        }
                    }
                    for page in ws.pages.iter() {
                        a {
                            href: "{page.url_path}",
                            style: "display: block; padding: 0.4rem 0.6rem; color: var(--qualia-accent, #0ff); text-decoration: none; border-radius: 4px; font-size: 0.85rem; transition: background 0.15s;",
                            onmouseenter: |_| {},
                            "{page.name}"
                        }
                    }
                }

                // Component Palette — draggable items grouped by category
                div {
                    h3 {
                        style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0 0 0.5rem 0;",
                        "Components"
                    }

                    // Group by category
                    {render_palette_category(&pane_palette.read(), PaneCategory::DataDisplay)}
                    {render_palette_category(&pane_palette.read(), PaneCategory::DataInput)}
                    {render_palette_category(&pane_palette.read(), PaneCategory::Layout)}
                    {render_palette_category(&pane_palette.read(), PaneCategory::Media)}
                    {render_palette_category(&pane_palette.read(), PaneCategory::System)}
                }
            }

            // ════════════════════════════════════════════════
            // CENTER: The Rendered Page Canvas (Drop Target)
            // ════════════════════════════════════════════════
            div {
                style: "background: var(--qualia-bg, #0a0a0a); padding: 1rem; overflow-y: auto;",
                ondrop: on_canvas_drop,
                ondragover: on_canvas_dragover,

                if let Some(page) = current_page.clone() {
                    div {
                        class: "{join_theme_classes(\"webizen-page-shell\", &page_theme)}",
                        "data-theme-scope": "page",
                        "data-theme": "{page_theme.theme_key.clone().unwrap_or_default()}",

                        // Page title and LLM Generation Bar (D2.1)
                        div {
                            style: "margin-bottom: 1rem; display: flex; justify-content: space-between; align-items: center; gap: 1rem;",
                            h2 {
                                style: "margin: 0; font-size: 1.1rem; color: var(--qualia-text, #eee); white-space: nowrap;",
                                "{page.name}"
                            }

                            // D2.1: LLM-Driven Pane Maker Prompt Bar
                            div {
                                style: "flex: 1; display: flex; gap: 0.5rem;",
                                input {
                                    type: "text",
                                    placeholder: "Describe a pane to generate (e.g., 'Health tracker with chart and inputs')...",
                                    style: "flex: 1; background: var(--qualia-surface, #222); border: 1px solid var(--qualia-border, #444); color: white; padding: 0.4rem 0.6rem; border-radius: 4px; font-size: 0.85rem;",
                                    // Simple mock handling for now
                                    onkeydown: |_evt| {
                                        // if evt.key() == "Enter" { ... fetch /generate_pane ... }
                                    }
                                }
                                button {
                                    style: "background: var(--qualia-accent, #0ff); color: black; border: none; padding: 0 1rem; border-radius: 4px; font-weight: bold; cursor: pointer;",
                                    "Generate"
                                }
                            }

                            span {
                                style: "font-size: 0.7rem; color: var(--qualia-text-muted, #666); background: var(--qualia-surface, #222); padding: 0.2rem 0.6rem; border-radius: 12px; white-space: nowrap;",
                                "{page.panes.len()} panes"
                            }
                        }

                        div {
                            style: "display: flex; flex-wrap: wrap; gap: 0.5rem; margin-bottom: 0.9rem;",
                            span {
                                style: "font-size: 0.68rem; color: var(--qualia-text, #ddd); background: rgba(255,255,255,0.05); border: 1px solid var(--qualia-border, #333); padding: 0.2rem 0.55rem; border-radius: 999px;",
                                "{presentation_mode_label(&page.presentation_mode)}"
                            }
                            span {
                                style: "font-size: 0.68rem; color: var(--qualia-text-muted, #aaa); background: rgba(255,255,255,0.03); border: 1px solid var(--qualia-border, #333); padding: 0.2rem 0.55rem; border-radius: 999px;",
                                "{coordinate_space_label(&page.coordinate_space)}"
                            }
                            span {
                                style: "font-size: 0.68rem; color: var(--qualia-text-muted, #aaa); background: rgba(255,255,255,0.03); border: 1px solid var(--qualia-border, #333); padding: 0.2rem 0.55rem; border-radius: 999px;",
                                "{layout_strategy_label(&page.layout_strategy)}"
                            }
                            if page.pan_and_zoom {
                                span {
                                    style: "font-size: 0.68rem; color: var(--qualia-accent, #0ff); background: rgba(6, 182, 212, 0.08); border: 1px solid rgba(6, 182, 212, 0.35); padding: 0.2rem 0.55rem; border-radius: 999px;",
                                    "Pan/Zoom Ready"
                                }
                            }
                        }

                        if !matches!(page.presentation_mode, PresentationMode::GridBound) {
                            div {
                                style: "margin-bottom: 0.9rem; padding: 0.65rem 0.8rem; border-radius: 10px; border: 1px solid var(--qualia-border, #333); background: rgba(255,255,255,0.03); color: var(--qualia-text-muted, #bbb); font-size: 0.78rem;",
                                "{presentation_mode_label(&page.presentation_mode)} pages are declared for this route. The current renderer keeps them on the point-grid canvas until the dedicated adapter is mounted."
                            }
                        }

                        // Workspace canvas
                        div {
                            style: "{canvas_container_style(&page)}",

                            for (idx, pane) in page.panes.iter().enumerate().filter(|(_, pane)| matches!(pane.layer, LayerBehavior::Docked)) {
                                {render_placed_pane(
                                    &page,
                                    pane,
                                    idx,
                                    &selected_pane_index,
                                    pane_themes.get(idx).cloned().unwrap_or_default(),
                                )}
                            }

                            if page.panes.iter().any(|pane| !matches!(pane.layer, LayerBehavior::Docked)) {
                                div {
                                    style: "position: absolute; inset: 0; pointer-events: none;",
                                    for (idx, pane) in page.panes.iter().enumerate().filter(|(_, pane)| !matches!(pane.layer, LayerBehavior::Docked)) {
                                        {render_placed_pane(
                                            &page,
                                            pane,
                                            idx,
                                            &selected_pane_index,
                                            pane_themes.get(idx).cloned().unwrap_or_default(),
                                        )}
                                    }
                                }
                            }

                            if page.panes.is_empty() {
                                div {
                                    style: "position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; min-height: 300px; border: 2px dashed var(--qualia-border, #333); border-radius: 12px; color: var(--qualia-text-muted, #555);",
                                    "Drag components from the palette to build your app"
                                }
                            }
                        }

                        if page.panes.iter().any(|pane| pane.min_w_points > 0 || pane.min_h_points > 0) {
                            div {
                                style: "margin-top: 0.75rem; font-size: 0.73rem; color: var(--qualia-text-muted, #777);",
                                "Point-grid panes can declare minimum working area and layered behavior independently of their current presentation mode."
                            }
                        }
                    }
                } else {
                    // No page found
                    div {
                        style: "display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 400px; color: var(--qualia-text-muted, #555);",
                        p { "No page mapped to this route." }
                        p { style: "font-size: 0.8rem;", "Navigate to / to see the Human-Centric Dashboard." }
                    }
                }
            }

            // ════════════════════════════════════════════════
            // RIGHT SIDEBAR: Inspector + Telemetry
            // ════════════════════════════════════════════════
            div {
                style: "background: var(--qualia-surface, #111); border-left: 1px solid var(--qualia-border, #333); padding: 1rem; overflow-y: auto; display: flex; flex-direction: column; gap: 1rem;",

                // Property Inspector
                div {
                    h3 {
                        style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0 0 0.5rem 0;",
                        "Property Inspector"
                    }

                    if let Some(idx) = *selected_pane_index.read() {
                        if let Some(page) = ws.pages.iter().find(|page| page.url_path == current_path) {
                            if let Some(pane) = page.panes.get(idx) {
                                div {
                                    style: "background: var(--qualia-bg, #0a0a0a); border-radius: 6px; padding: 0.75rem; font-size: 0.8rem;",
                                    div {
                                        style: "margin-bottom: 0.5rem;",
                                        span { style: "color: var(--qualia-text-muted, #888);", "Component: " }
                                        span { style: "color: var(--qualia-accent, #0ff);", "{pane.component_id}" }
                                    }
                                    div {
                                        style: "margin-bottom: 0.5rem;",
                                        span { style: "color: var(--qualia-text-muted, #888);", "Position: " }
                                        span { "({pane.x}, {pane.y}) — {pane.w}×{pane.h}" }
                                    }
                                    div {
                                        style: "margin-bottom: 0.5rem;",
                                        span { style: "color: var(--qualia-text-muted, #888);", "Layer: " }
                                        span { "{layer_behavior_label(&pane.layer)}" }
                                    }
                                    if pane.min_w_points > 0 || pane.min_h_points > 0 {
                                        div {
                                            style: "margin-bottom: 0.5rem;",
                                            span { style: "color: var(--qualia-text-muted, #888);", "Minimum area: " }
                                            span { "{pane.min_w_points}x{pane.min_h_points} points" }
                                        }
                                    }
                                    if !pane.supported_presentations.is_empty() {
                                        div {
                                            style: "margin-bottom: 0.5rem;",
                                            span { style: "color: var(--qualia-text-muted, #888);", "Supported views: " }
                                            span { "{supported_presentations_summary(&pane.supported_presentations)}" }
                                        }
                                    }
                                    div {
                                        span { style: "color: var(--qualia-text-muted, #888);", "Bindings: " }
                                        span {
                                            if pane.data_bindings.is_empty() {
                                                "None"
                                            } else {
                                                "{pane.data_bindings.join(\", \")}"
                                            }
                                        }
                                    }
                                    if let Some(rpc) = &pane.binds_rpc {
                                        div {
                                            style: "margin-top: 0.5rem; padding-top: 0.5rem; border-top: 1px solid var(--qualia-border, #444);",
                                            span { style: "color: var(--qualia-text-muted, #888);", "RPC: " }
                                            span { style: "color: #ffaa00;", "{rpc}" }
                                        }
                                        div {
                                            span { style: "color: var(--qualia-text-muted, #888);", "UI Mode: " }
                                            span {
                                                if pane.ui_mode == Some(UiMode::IFrameSandbox) { "IFrame Sandbox" } else { "Native Dioxus" }
                                            }
                                        }
                                    }
                                }

                                button {
                                    style: "margin-top: 0.5rem; width: 100%; padding: 0.4rem; background: var(--qualia-danger, #c00); color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.8rem;",
                                    onclick: delete_selected_pane,
                                    "Remove Pane"
                                }
                            }
                        }
                    } else {
                        p {
                            style: "color: var(--qualia-text-muted, #555); font-size: 0.8rem;",
                            "Click a pane on the canvas to inspect its properties."
                        }
                    }
                }

                // SPARQL Binding Area
                div {
                    h3 {
                        style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0 0 0.5rem 0;",
                        "Data Binding"
                    }
                    div {
                        style: "background: var(--qualia-bg, #0a0a0a); padding: 0.5rem; border-radius: 4px;",
                        p { style: "font-size: 0.75rem; color: var(--qualia-text-muted, #666);", "SPARQL / N3Logic query binding" }
                        code { style: "font-size: 0.7rem; color: var(--qualia-success, #0f0);", "Parity: {mock_quin.read().parity}" }
                    }
                }

                // Live Telemetry
                div {
                    h3 {
                        style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0 0 0.5rem 0;",
                        "Live Telemetry"
                    }
                    div {
                        style: "height: 120px; overflow-y: auto; background: var(--qualia-bg, #000); color: var(--qualia-success, #0f0); padding: 0.5rem; font-family: monospace; font-size: 0.7rem; border-radius: 4px;",
                        for log in telemetry_logs.read().iter() {
                            div { "{log}" }
                        }
                        if telemetry_logs.read().is_empty() {
                            div { style: "color: #444;", "Waiting for telemetry stream..." }
                        }
                    }
                }

                // LLM Engine Panel
                div {
                    h3 {
                        style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0 0 0.5rem 0;",
                        "Native LLM Engine"
                    }
                    if *is_native_llm_active.read() {
                        div {
                            style: "background: rgba(16, 185, 129, 0.1); border: 1px solid #10b981; padding: 0.5rem; border-radius: 4px;",
                            span { style: "color: #10b981; font-size: 0.75rem; font-weight: bold;", "● Webizen Server Connected" }
                            p { style: "color: var(--qualia-text-muted); font-size: 0.7rem; margin: 0.3rem 0 0 0;", "Native offload is available for deeper inference, while the WASM workspace continues to run locally." }
                        }
                    } else {
                        div {
                            style: "background: rgba(255, 170, 0, 0.1); border: 1px solid #ffaa00; padding: 0.5rem; border-radius: 4px;",
                            span { style: "color: #ffaa00; font-size: 0.75rem; font-weight: bold;", "○ Standalone WASM Mode" }
                            p { style: "color: var(--qualia-text-muted); font-size: 0.7rem; margin: 0.3rem 0 0 0;", "The studio still runs locally in-browser; launch a Webizen Server only when you want native offload." }
                        }
                    }
                }

                // Deploy Button
                button {
                    style: "margin-top: auto; width: 100%; padding: 0.6rem; background: linear-gradient(135deg, var(--qualia-accent, #0ff), var(--qualia-primary, #06f)); color: white; border: none; border-radius: 6px; cursor: pointer; font-weight: bold; font-size: 0.85rem; letter-spacing: 0.05em; transition: opacity 0.2s;",
                    onclick: deploy_workspace,
                    "Deploy to Network"
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Helper: render a palette category section with draggable items
// ─────────────────────────────────────────────────────────────

fn render_palette_category(palette: &[PaneDefinition], category: PaneCategory) -> Element {
    let items: Vec<&PaneDefinition> = palette.iter().filter(|p| p.category == category).collect();
    if items.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            style: "margin-bottom: 0.75rem;",
            div {
                style: "font-size: 0.65rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--qualia-text-muted, #666); margin-bottom: 0.3rem;",
                "{category_label(&category)}"
            }
            for item in items.iter() {
                div {
                    style: "padding: 0.35rem 0.5rem; margin-bottom: 2px; background: var(--qualia-bg, #0a0a0a); border: 1px solid var(--qualia-border, #2a2a2a); border-radius: 4px; cursor: grab; font-size: 0.8rem; display: flex; align-items: center; gap: 0.4rem; transition: border-color 0.15s, background 0.15s; user-select: none;",
                    draggable: "true",
                    "data-component-id": "{item.component_id}",
                    ondragstart: {
                        let cid = item.component_id.clone();
                        move |evt: Event<DragData>| {
                            let dt = evt.data().data_transfer();
                            let _ = dt.set_data("application/x-qualia-pane-id", &cid);
                        }
                    },
                    // Icon placeholder
                    span {
                        style: "width: 14px; height: 14px; border-radius: 3px; background: var(--qualia-accent, #0ff); opacity: 0.5; flex-shrink: 0;",
                    }
                    span { "{item.display_name}" }
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Helper: render a placed pane on the canvas with selection
// ─────────────────────────────────────────────────────────────

fn presentation_mode_label(mode: &PresentationMode) -> &'static str {
    match mode {
        PresentationMode::GridBound => "Grid View",
        PresentationMode::NodeRelational => "Node View",
        PresentationMode::Spatial => "Spatial View",
    }
}

fn coordinate_space_label(space: &CoordinateSpace) -> &'static str {
    match space {
        CoordinateSpace::GlobalCartesian => "Global Coordinates",
        CoordinateSpace::RelativeAnchored => "Relative Anchors",
    }
}

fn layout_strategy_label(layout: &LayoutStrategy) -> &'static str {
    match layout {
        LayoutStrategy::PointGrid { .. } => "Point Grid",
        LayoutStrategy::CssGrid { .. } => "Legacy CSS Grid",
        LayoutStrategy::FlexBox => "Flex Layout",
        LayoutStrategy::Masonry => "Masonry Layout",
    }
}

fn layer_behavior_label(layer: &LayerBehavior) -> &'static str {
    match layer {
        LayerBehavior::Docked => "Docked",
        LayerBehavior::FloatingOverlay => "Floating Overlay",
        LayerBehavior::ModalOverlay => "Modal Overlay",
        LayerBehavior::FullCanvas => "Full Canvas",
    }
}

fn supported_presentations_summary(modes: &[PresentationMode]) -> String {
    modes
        .iter()
        .map(presentation_mode_label)
        .collect::<Vec<_>>()
        .join(", ")
}

fn canvas_container_style(page: &Page) -> String {
    match &page.layout_strategy {
        LayoutStrategy::PointGrid {
            width_points,
            height_points,
            snap_step,
            gutter: _,
        } => format!(
            "position: relative; min-height: 520px; height: min(76vh, {}px); overflow: hidden; border: 1px solid var(--qualia-border, #333); border-radius: 16px; background-color: rgba(10,10,10,0.45); background-image: linear-gradient(rgba(255,255,255,0.05) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.05) 1px, transparent 1px); background-size: calc(100% / {}) calc(100% / {});",
            (*height_points as u32).saturating_mul(10),
            width_points.max(snap_step),
            height_points.max(snap_step),
        ),
        LayoutStrategy::CssGrid { cols, rows, gap } => format!(
            "display: grid; position: relative; grid-template-columns: repeat({}, 1fr); grid-template-rows: repeat({}, 80px); gap: {}px; min-height: 400px;",
            cols, rows, gap
        ),
        LayoutStrategy::FlexBox => {
            "display: flex; position: relative; flex-direction: column; gap: 1rem; min-height: 400px;"
                .to_string()
        }
        LayoutStrategy::Masonry => {
            "display: block; position: relative; min-height: 400px; column-width: 280px; column-gap: 1rem;"
                .to_string()
        }
    }
}

fn pane_style_for_layout(page: &Page, pane: &PanePlacement, is_selected: bool) -> String {
    let border_color = if is_selected {
        "var(--qualia-accent, #0ff)"
    } else {
        "var(--qualia-border, #333)"
    };
    let bg = if is_selected {
        "rgba(0, 255, 255, 0.08)"
    } else {
        "var(--qualia-surface, #181818)"
    };
    let shadow = if is_selected {
        "0 0 0 1px rgba(6, 182, 212, 0.25), 0 16px 36px rgba(0, 0, 0, 0.28)"
    } else {
        "0 12px 26px rgba(0, 0, 0, 0.18)"
    };

    match (&page.layout_strategy, &pane.layer) {
        (LayoutStrategy::CssGrid { .. }, LayerBehavior::Docked) => format!(
            "grid-column: {} / span {}; grid-row: {} / span {}; background: {}; border: 1px solid {}; border-radius: 10px; padding: 0.75rem; cursor: pointer; transition: border-color 0.2s, background 0.2s, transform 0.2s; display: flex; flex-direction: column; justify-content: space-between; min-height: 88px; box-shadow: {};",
            pane.x.max(1),
            pane.w.max(1),
            pane.y.max(1),
            pane.h.max(1),
            bg,
            border_color,
            shadow,
        ),
        _ => {
            let (grid_w, grid_h, gutter) = match &page.layout_strategy {
                LayoutStrategy::PointGrid {
                    width_points,
                    height_points,
                    gutter,
                    ..
                } => (*width_points as f32, *height_points as f32, *gutter),
                _ => (100.0, 100.0, 2),
            };
            let left = (pane.x as f32 / grid_w).clamp(0.0, 1.0) * 100.0;
            let top = (pane.y as f32 / grid_h).clamp(0.0, 1.0) * 100.0;
            let width = (pane.w.max(1) as f32 / grid_w).clamp(0.08, 1.0) * 100.0;
            let height = (pane.h.max(1) as f32 / grid_h).clamp(0.08, 1.0) * 100.0;
            let min_width_px = pane.min_w_points.max(12) as u32 * 6;
            let min_height_px = pane.min_h_points.max(10) as u32 * 5;

            match pane.layer {
                LayerBehavior::Docked => format!(
                    "position: absolute; left: {:.3}%; top: {:.3}%; width: calc({:.3}% - {}px); height: calc({:.3}% - {}px); min-width: {}px; min-height: {}px; background: {}; border: 1px solid {}; border-radius: 12px; padding: 0.75rem; cursor: pointer; transition: border-color 0.2s, background 0.2s, transform 0.2s; display: flex; flex-direction: column; justify-content: space-between; box-shadow: {}; overflow: hidden;",
                    left, top, width, gutter, height, gutter, min_width_px, min_height_px, bg, border_color, shadow,
                ),
                LayerBehavior::FloatingOverlay => format!(
                    "position: absolute; pointer-events: auto; left: {:.3}%; top: {:.3}%; width: calc({:.3}% - {}px); height: calc({:.3}% - {}px); min-width: {}px; min-height: {}px; background: color-mix(in srgb, {} 92%, black 8%); border: 1px solid {}; border-radius: 14px; padding: 0.75rem; cursor: pointer; display: flex; flex-direction: column; justify-content: space-between; box-shadow: 0 22px 50px rgba(0, 0, 0, 0.35); backdrop-filter: blur(14px); z-index: 30; overflow: hidden;",
                    left, top, width, gutter, height, gutter, min_width_px, min_height_px, bg, border_color,
                ),
                LayerBehavior::ModalOverlay => format!(
                    "position: absolute; pointer-events: auto; left: 50%; top: 50%; width: min({:.3}%, 760px); height: min({:.3}%, 560px); min-width: {}px; min-height: {}px; transform: translate(-50%, -50%); background: color-mix(in srgb, {} 94%, black 6%); border: 1px solid {}; border-radius: 16px; padding: 0.9rem; cursor: pointer; display: flex; flex-direction: column; justify-content: space-between; box-shadow: 0 28px 80px rgba(0, 0, 0, 0.45); backdrop-filter: blur(16px); z-index: 45; overflow: hidden;",
                    width.max(28.0), height.max(24.0), min_width_px.max(280), min_height_px.max(220), bg, border_color,
                ),
                LayerBehavior::FullCanvas => format!(
                    "position: absolute; pointer-events: auto; inset: 0; background: {}; border: 1px solid {}; border-radius: 14px; padding: 0.9rem; cursor: pointer; display: flex; flex-direction: column; justify-content: space-between; box-shadow: 0 22px 50px rgba(0, 0, 0, 0.32); z-index: 55; overflow: hidden;",
                    bg, border_color,
                ),
            }
        }
    }
}

fn render_placed_pane(
    page: &Page,
    pane: &PanePlacement,
    idx: usize,
    selected: &Signal<Option<usize>>,
    theme: ResolvedTheme,
) -> Element {
    let is_selected = *selected.read() == Some(idx);

    // Look up the display name from the registry
    let display_name = find_pane(&pane.component_id)
        .map(|p| p.display_name)
        .unwrap_or_else(|| pane.component_id.clone());

    let element_tag = find_pane(&pane.component_id)
        .map(|p| p.element_tag.clone())
        .unwrap_or_default();

    let mut selected = selected.clone();

    rsx! {
        div {
            class: "{join_theme_classes(\"webizen-module-pane\", &theme)}",
            "data-theme-scope": "module",
            "data-theme": "{theme.theme_key.clone().unwrap_or_default()}",
            "data-pane-index": "{idx}",
            style: "{pane_style_for_layout(page, pane, is_selected)}",
            onclick: move |_| {
                selected.set(Some(idx));
            },

            // Component label
            div {
                style: "display: flex; justify-content: space-between; align-items: center;",
                div {
                    style: "display: flex; flex-direction: column; gap: 0.2rem;",
                    span {
                        style: "font-size: 0.8rem; font-weight: 600; color: var(--qualia-text, #ddd);",
                        "{display_name}"
                    }
                    span {
                        style: "font-size: 0.62rem; color: var(--qualia-text-muted, #777); text-transform: uppercase; letter-spacing: 0.08em;",
                        "{layer_behavior_label(&pane.layer)}"
                    }
                }
                span {
                    style: "font-size: 0.6rem; color: var(--qualia-text-muted, #555); background: var(--qualia-bg, #0a0a0a); padding: 0.1rem 0.4rem; border-radius: 8px;",
                    "{element_tag}"
                }
            }

            // Semantic indicator
            div {
                style: "font-size: 0.65rem; color: var(--qualia-text-muted, #555); margin-top: auto;",
                if let Some(anchor) = pane.anchor.as_ref() {
                    "Anchored to {anchor}"
                } else if !pane.data_bindings.is_empty() {
                    "📊 Bound to {pane.data_bindings.len()} queries"
                } else {
                    "No data binding"
                }
            }
        }
    }
}

