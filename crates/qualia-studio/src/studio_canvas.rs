use dioxus::prelude::*;
use qualia_core_db::NQuin;
use serde::{Deserialize, Serialize};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{EventSource, MessageEvent};

use crate::pane_registry::{
    builtin_pane_definitions, category_label, find_pane, PaneCategory, PaneDefinition,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum LayoutStrategy {
    CssGrid { cols: u8, rows: u8, gap: u8 },
    FlexBox,
    Masonry,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PanePlacement {
    pub component_id: String,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
    pub data_bindings: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Page {
    pub url_path: String,
    pub name: String,
    pub layout_strategy: LayoutStrategy,
    pub panes: Vec<PanePlacement>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct WebizenWorkspace {
    pub pages: Vec<Page>,
    pub theme_tokens: std::collections::HashMap<String, String>,
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
    let mut selected_pane_index = use_signal(|| None::<usize>);
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

    // ── Telemetry SSE ──────────────────────────────────────
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
                        component_id,
                        x: 1,
                        y: (workspace.read().pages.first().map(|p| p.panes.len()).unwrap_or(0) as u8) + 1,
                        w: default_w,
                        h: default_h,
                        data_bindings: vec![],
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
    let current_page = ws.pages.iter().find(|p| p.url_path == current_path);

    rsx! {
        div {
            style: "flex: 1; display: grid; grid-template-columns: 240px 1fr 280px; gap: 0; height: calc(100vh - 60px);",
            
            // D3.1: App-Scoped Theme Tokens
            style {
                ":root {{\n"
                for (key, val) in ws.theme_tokens.iter() {
                    "--qualia-{key}: {val};\n"
                }
                "}}\n"
            }

            // ════════════════════════════════════════════════
            // LEFT SIDEBAR: Pages + Component Palette
            // ════════════════════════════════════════════════
            div {
                style: "background: var(--qualia-surface, #111); border-right: 1px solid var(--qualia-border, #333); padding: 1rem; overflow-y: auto; display: flex; flex-direction: column; gap: 0.75rem;",

                // Page Navigation
                div {
                    style: "margin-bottom: 0.5rem;",
                    h3 {
                        style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0 0 0.5rem 0;",
                        "Pages"
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

                if let Some(page) = current_page {
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

                    // Grid canvas
                    div {
                        style: match &page.layout_strategy {
                            LayoutStrategy::CssGrid { cols, rows, gap } => format!(
                                "display: grid; grid-template-columns: repeat({}, 1fr); grid-template-rows: repeat({}, 80px); gap: {}px; min-height: 400px;",
                                cols, rows, gap
                            ),
                            LayoutStrategy::FlexBox => "display: flex; flex-direction: column; gap: 1rem; min-height: 400px;".to_string(),
                            _ => "display: block; min-height: 400px;".to_string(),
                        },

                        for (idx, pane) in page.panes.iter().enumerate() {
                            {render_placed_pane(pane, idx, &selected_pane_index)}
                        }
                    }

                    // Drop hint when empty
                    if page.panes.is_empty() {
                        div {
                            style: "display: flex; align-items: center; justify-content: center; min-height: 300px; border: 2px dashed var(--qualia-border, #333); border-radius: 12px; color: var(--qualia-text-muted, #555);",
                            "Drag components from the palette to build your app"
                        }
                    }
                } else {
                    // No page found
                    div {
                        style: "display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 400px; color: var(--qualia-text-muted, #555);",
                        p { "No page mapped to this route." }
                        p { style: "font-size: 0.8rem;", "Navigate to / to see the Fiduciary Dashboard." }
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
                        if let Some(page) = ws.pages.first() {
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
                                        span { style: "color: var(--qualia-text-muted, #888);", "Bindings: " }
                                        span {
                                            if pane.data_bindings.is_empty() {
                                                "None"
                                            } else {
                                                "{pane.data_bindings.join(\", \")}"
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

fn render_placed_pane(pane: &PanePlacement, idx: usize, selected: &Signal<Option<usize>>) -> Element {
    let is_selected = *selected.read() == Some(idx);
    let border_color = if is_selected { "var(--qualia-accent, #0ff)" } else { "#333" };
    let bg = if is_selected { "rgba(0, 255, 255, 0.05)" } else { "var(--qualia-surface, #181818)" };

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
            style: format!(
                "grid-column: {} / span {}; grid-row: {} / span {}; background: {}; border: 1px solid {}; border-radius: 6px; padding: 0.75rem; cursor: pointer; transition: border-color 0.2s, background 0.2s; display: flex; flex-direction: column; justify-content: space-between;",
                pane.x, pane.w, pane.y, pane.h, bg, border_color
            ),
            onclick: move |_| {
                selected.set(Some(idx));
            },

            // Component label
            div {
                style: "display: flex; justify-content: space-between; align-items: center;",
                span {
                    style: "font-size: 0.8rem; font-weight: 600; color: var(--qualia-text, #ddd);",
                    "{display_name}"
                }
                span {
                    style: "font-size: 0.6rem; color: var(--qualia-text-muted, #555); background: var(--qualia-bg, #0a0a0a); padding: 0.1rem 0.4rem; border-radius: 8px;",
                    "{element_tag}"
                }
            }

            // Semantic indicator
            div {
                style: "font-size: 0.65rem; color: var(--qualia-text-muted, #555); margin-top: auto;",
                if !pane.data_bindings.is_empty() {
                    "📊 Bound to {pane.data_bindings.len()} queries"
                } else {
                    "No data binding"
                }
            }
        }
    }
}
