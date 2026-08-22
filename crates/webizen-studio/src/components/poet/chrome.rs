//! Menubar, pager, strata, epistemic, 4D ribbon, exposé, status — NLP chrome.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::kinds::{DimMode, Epistemic, ManifoldId, Strata};
use super::store::Workbench;
use dioxus::prelude::*;

#[component]
pub fn TopMenubar(wb: Signal<Workbench>) -> Element {
    let open = wb().menu;
    rsx! {
        header { class: "top-menubar",
            div { class: "menu-items-group",
                span { class: "brand-icon", "🌌" }
                span { style: "font-weight:700;font-size:13px;letter-spacing:0.04em;color:var(--accent-cyan);margin-right:8px;", "POET" }
                Menu { name: "file", label: "File", open, wb,
                    Item { left: "New Manifold Desk", right: "Ctrl+N" }
                    Item { left: "Open HyperDoc / Desk...", right: "Ctrl+O" }
                    Item { left: "Import .hcf Container", right: "" }
                    Divider {}
                    Item { left: "Save to Merkle DAG", right: "Ctrl+S" }
                    Item { left: "Export RDF 1.2 Triples", right: "" }
                    Item { left: "Export Solid Pod Turtle", right: "" }
                }
                Menu { name: "canvases", label: "Manifolds", open, wb,
                    CanvasItem { wb, id: ManifoldId::Research, hint: "Alt+1" }
                    CanvasItem { wb, id: ManifoldId::Media, hint: "Alt+2" }
                    CanvasItem { wb, id: ManifoldId::Social, hint: "Alt+3" }
                    CanvasItem { wb, id: ManifoldId::Mail, hint: "Alt+4" }
                    CanvasItem { wb, id: ManifoldId::Chora, hint: "Alt+5" }
                    CanvasItem { wb, id: ManifoldId::Settings, hint: "Alt+6" }
                    Divider {}
                    div { class: "dropdown-item",
                        onclick: move |_| { let mut w = wb(); w.expose = !w.expose; w.menu = None; wb.set(w); },
                        span { "🗂️ Exposé Overview (All Desktops)" }
                        span { class: "shortcut-hint", "Alt+O" }
                    }
                }
                Menu { name: "view", label: "View", open, wb,
                    div { class: "dropdown-item", onclick: move |_| set_dim(wb, DimMode::D2), span { "2D Flat Canvas" } }
                    div { class: "dropdown-item", onclick: move |_| set_dim(wb, DimMode::D3), span { "3D Perspective Orbit" } }
                    div { class: "dropdown-item", onclick: move |_| set_dim(wb, DimMode::D4), span { "4D Spatiotemporal Plane" } }
                }
                Menu { name: "insert", label: "Insert", open, wb,
                    span { style: "padding:6px 14px;color:var(--text-muted);font-size:11px;", "Use the tool chest to place containers." }
                }
            }
            div { class: "menu-items-group", style: "display:flex;align-items:center;gap:12px;",
                // Ambient Job Indicator
                div {
                    class: "ambient-job-indicator",
                    style: "display:flex;align-items:center;gap:6px;padding:3px 10px;background:rgba(0,200,255,0.08);border:1px solid rgba(0,200,255,0.25);border-radius:12px;font-size:11px;color:var(--accent-cyan);",
                    span { style: "display:inline-block;width:7px;height:7px;border-radius:50%;background:#00E676;box-shadow:0 0 6px #00E676;" }
                    span { "Mesh Active · 42MB Sentinel OK" }
                }
                span { style: "font-family:var(--font-mono);font-size:11px;color:var(--text-muted);", "Poet HyperCanvas · vibe-0.1" }
            }
        }
    }
}

#[component]
fn Menu(
    name: &'static str,
    label: &'static str,
    open: Option<&'static str>,
    wb: Signal<Workbench>,
    children: Element,
) -> Element {
    let shown = open == Some(name);
    rsx! {
        button {
            class: if shown { "menu-btn active" } else { "menu-btn" },
            onclick: move |e| {
                e.stop_propagation();
                let mut w = wb();
                w.menu = if w.menu == Some(name) { None } else { Some(name) };
                wb.set(w);
            },
            "{label}"
            if shown {
                div { class: "dropdown-menu show", id: "menu-{name}",
                    {children}
                }
            }
        }
    }
}

#[component]
fn Item(left: &'static str, right: &'static str) -> Element {
    rsx! {
        div { class: "dropdown-item",
            span { "{left}" }
            if !right.is_empty() { span { class: "shortcut-hint", "{right}" } }
        }
    }
}

#[component]
fn Divider() -> Element {
    rsx! { div { class: "dropdown-divider" } }
}

#[component]
fn CanvasItem(wb: Signal<Workbench>, id: ManifoldId, hint: &'static str) -> Element {
    rsx! {
        div { class: "dropdown-item",
            onclick: move |_| { let mut w = wb(); w.switch(id); wb.set(w); },
            span { "{id.icon()} {id.title()}" }
            span { class: "shortcut-hint", "{hint}" }
        }
    }
}

fn set_dim(mut wb: Signal<Workbench>, dim: DimMode) {
    let mut w = wb();
    w.dim = dim;
    w.menu = None;
    wb.set(w);
}

#[component]
pub fn ControlBar(wb: Signal<Workbench>) -> Element {
    let w = wb();
    rsx! {
        div { class: "canvas-control-bar",
            div { class: "virtual-desktop-pager",
                button { class: "pager-expose-btn",
                    onclick: move |_| { let mut s = wb(); s.expose = !s.expose; wb.set(s); },
                    "🗂️ Overview"
                }
                div { class: "pager-desktops-list",
                    for (i, id) in ManifoldId::ALL.iter().enumerate() {
                        button {
                            class: if w.active == *id { "desktop-tab-btn active" } else { "desktop-tab-btn" },
                            title: "{id.title()}",
                            onclick: move |_| { let mut s = wb(); s.switch(*id); wb.set(s); },
                            span { class: "desktop-num", "{i + 1}" }
                            span { "{id.icon()}" }
                            span { "{id.short()}" }
                        }
                    }
                }
                button { class: "pager-add-btn", title: "Create New Manifold Workspace", "+" }
            }
            div { class: "canvas-title-box",
                input { class: "canvas-title-input", value: "{w.title}", readonly: true }
                span { class: "graph-address-badge", "{w.graph_iri}" }
            }
            div { class: "strata-deck-selector",
                button {
                    class: if w.strata.len() >= 5 { "strata-deck-btn active" } else { "strata-deck-btn" },
                    "data-strata": "all",
                    onclick: move |_| { let mut s = wb(); s.select_all_strata(); wb.set(s); },
                    "All Strata"
                }
                for s in Strata::ALL {
                    button {
                        class: if w.strata_on(s) { "strata-deck-btn active" } else { "strata-deck-btn" },
                        "data-strata": "{s.id()}",
                        onclick: move |_| { let mut st = wb(); st.toggle_strata(s); wb.set(st); },
                        "{s.label()}"
                    }
                }
            }
            div { class: "epistemic-lens-selector",
                for epi in [Epistemic::All, Epistemic::Objective, Epistemic::Subjective, Epistemic::Intersubjective] {
                    button {
                        class: if w.epistemic == epi { "epistemic-btn active" } else { "epistemic-btn" },
                        "data-epistemic": "{epi.id()}",
                        onclick: move |_| { let mut st = wb(); st.epistemic = epi; wb.set(st); },
                        if epi == Epistemic::All { "All Modalities" }
                        else { "{epi.icon()} {epi.id()}" }
                    }
                }
            }
            div { class: "dimension-switch",
                for dim in [DimMode::D2, DimMode::D3, DimMode::D4] {
                    button {
                        class: if w.dim == dim { "dim-btn active" } else { "dim-btn" },
                        onclick: move |_| { let mut st = wb(); st.dim = dim; wb.set(st); },
                        if dim == DimMode::D2 { "2D" }
                        else if dim == DimMode::D3 { "3D Orbit" }
                        else { "4D Time" }
                    }
                }
            }
            div { class: "datetime-span-ribbon",
                span { class: "datetime-badge", "08-15 00:00" }
                div { class: "time-slider-container",
                    button { class: "play-pause-btn",
                        onclick: move |_| { let mut st = wb(); st.playing = !st.playing; wb.set(st); },
                        if w.playing { "⏸" } else { "▶" }
                    }
                    input {
                        r#type: "range", class: "time-slider", min: "0", max: "100",
                        value: "{(w.time_progress * 100.0) as i32}",
                        oninput: move |e| {
                            if let Ok(v) = e.value().parse::<f64>() {
                                let mut st = wb();
                                st.time_progress = v / 100.0;
                                wb.set(st);
                            }
                        },
                    }
                    span { class: "datetime-badge active-time", "14:40:00" }
                }
                span { class: "datetime-badge", "08-15 23:59" }
            }
            button { class: "btn",
                style: "background:var(--surface-panel);border:1px solid var(--border-medium);color:var(--text-primary);font-size:11px;padding:4px 8px;border-radius:var(--radius-sm);cursor:pointer;",
                onclick: move |_| { let mut st = wb(); st.sidebar = !st.sidebar; wb.set(st); },
                "⚙️ Telemetry & DAG"
            }
        }
    }
}

#[component]
pub fn StatusBar(wb: Signal<Workbench>) -> Element {
    let w = wb();
    let node = w.selected.clone().unwrap_or_else(|| "none".into());
    let strata = if w.strata.len() >= 5 {
        "All (5 Levels Active)".into()
    } else {
        w.strata
            .iter()
            .map(|s| s.id())
            .collect::<Vec<_>>()
            .join(", ")
    };
    rsx! {
        footer { class: "bottom-statusbar",
            div { style: "display:flex;gap:16px;",
                span { strong { "Active Node:" } " " span { style: "color:var(--accent-cyan);", "{node}" } }
                span { strong { "Strata:" } " " span { style: "color:var(--accent-emerald);", "{strata}" } }
                span { strong { "Epistemic Lens:" } " " span { style: "color:var(--modality-objective);", "{w.epistemic.id()}" } }
            }
            div { style: "display:flex;gap:16px;",
                span { strong { "Identity:" } " did:qualia:timothy_charles_holborn" }
                span { strong { "Fiduciary Gate:" } " " span { style: "color:var(--accent-emerald);", "Level 3 Inalienable Custody" } }
            }
        }
    }
}

#[component]
pub fn Expose(wb: Signal<Workbench>) -> Element {
    if !wb().expose {
        return rsx! {};
    }
    rsx! {
        div { class: "expose-overview-grid", style: "display:flex;",
            div { class: "expose-header",
                h2 { "🗂️ Manifold Overview" }
                button { class: "pager-expose-btn",
                    onclick: move |_| { let mut s = wb(); s.expose = false; wb.set(s); },
                    "Close"
                }
            }
            div { class: "expose-grid",
                for id in ManifoldId::ALL {
                    div {
                        class: if wb().active == id { "expose-card active" } else { "expose-card" },
                        onclick: move |_| { let mut s = wb(); s.switch(id); s.expose = false; wb.set(s); },
                        div { class: "expose-card-header",
                            span { "{id.icon()} {id.short()}" }
                            span { class: "graph-address-badge", "{id.graph_iri()}" }
                        }
                        div { class: "expose-card-preview",
                            p { style: "color:var(--text-secondary);font-size:12px;", "{id.title()}" }
                        }
                    }
                }
            }
        }
    }
}
