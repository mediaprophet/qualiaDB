//! HyperCanvas shell — structure from `Canvas_Workbench/index.html` and `POET-SPEC-001..023`.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::chest::ToolChest;
use super::chrome::{ControlBar, Expose, StatusBar, TopMenubar};
use super::radial_menu::{RadialActionRing, RadialState};
use super::stage::CanvasStage;
use super::store::Workbench;
use super::styles::HyperCanvasStyles;
use dioxus::prelude::*;

#[component]
pub fn PoetWorkbench() -> Element {
    let mut wb = use_signal(Workbench::new);
    let mut radial = use_signal(RadialState::default);

    rsx! {
        HyperCanvasStyles {}
        div {
            id: "app-root",
            onclick: move |_| {
                let mut s = wb();
                if s.menu.is_some() {
                    s.menu = None;
                    wb.set(s);
                }
                let mut rd = radial();
                if rd.visible {
                    rd.visible = false;
                    radial.set(rd);
                }
            },
            oncontextmenu: move |e| {
                e.prevent_default();
                let coords = e.data().client_coordinates();
                radial.set(RadialState {
                    visible: true,
                    x: coords.x,
                    y: coords.y,
                });
            },
            onkeydown: move |e| {
                let key = e.data().key().to_string();
                if key == "o" && e.data().modifiers().alt() {
                    let mut s = wb();
                    s.expose = !s.expose;
                    wb.set(s);
                }
            },
            TopMenubar { wb }
            ControlBar { wb }
            div { class: "main-workspace",
                ToolChest { wb }
                CanvasStage { wb }
                aside {
                    class: if wb().sidebar { "tech-sidebar open" } else { "tech-sidebar" },
                    div { style: "padding:14px;display:grid;gap:8px;",
                        h3 { style: "margin:0;font-size:13px;color:var(--accent-cyan);", "Telemetry & Governance DAG" }
                        p { style: "margin:0;color:var(--text-secondary);font-size:12px;line-height:1.45;",
                            "Merkle / Pulse bus is Active. Graph address: {wb().graph_iri}. Nodes on desk: {wb().nodes.len()}."
                        }
                        div { style: "margin-top:8px;padding:8px;background:rgba(0,0,0,0.3);border:1px solid rgba(255,255,255,0.06);border-radius:6px;font-size:11px;",
                            div { style: "color:var(--accent-emerald);", "● 42MB Prolog Sentinel: ENFORCED" }
                            div { style: "color:var(--text-muted);margin-top:4px;", "Zero-Heap Hot-Path: Active" }
                            div { style: "color:var(--text-muted);margin-top:2px;", "Grid: 8px Snap Math" }
                        }
                    }
                }
            }
            StatusBar { wb }
            Expose { wb }
            RadialActionRing { wb, state: radial }
        }
    }
}
