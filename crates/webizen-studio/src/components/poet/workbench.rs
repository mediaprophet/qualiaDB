//! HyperCanvas shell — structure from `Canvas_Workbench/index.html`.

use super::chest::ToolChest;
use super::chrome::{ControlBar, Expose, StatusBar, TopMenubar};
use super::stage::CanvasStage;
use super::store::Workbench;
use super::styles::HyperCanvasStyles;
use dioxus::prelude::*;

#[component]
pub fn PoetWorkbench() -> Element {
    let mut wb = use_signal(Workbench::new);

    rsx! {
        HyperCanvasStyles {}
        div { id: "app-root",
            onclick: move |_| {
                let mut s = wb();
                if s.menu.is_some() {
                    s.menu = None;
                    wb.set(s);
                }
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
                        h3 { style: "margin:0;font-size:13px;", "Telemetry & DAG" }
                        p { style: "margin:0;color:var(--text-secondary);font-size:12px;line-height:1.45;",
                            "Merkle / Pulse bus is Partial. Graph address: {wb().graph_iri}. Nodes on this desk: {wb().nodes.len()}."
                        }
                    }
                }
            }
            StatusBar { wb }
            Expose { wb }
        }
    }
}
