//! One manifold surface: GPU desk + the containers currently on it.

use super::containers::ContainerCard;
use super::gpu_frame::PoetGpuFrame;
use super::kinds::{ContainerInstance, ManifoldId};
use dioxus::prelude::*;

#[component]
pub fn ManifoldView(active: ManifoldId, items: Vec<ContainerInstance>) -> Element {
    let here: Vec<ContainerInstance> = items.into_iter().filter(|c| c.on == active).collect();
    rsx! {
        section {
            style: "position:relative;height:100%;min-height:420px;overflow:hidden;background:#05070c;",
            PoetGpuFrame { kind: active.id(), width: 1280, height: 720 }
            div {
                style: "position:absolute;inset:0;display:grid;grid-template-rows:auto 1fr;pointer-events:none;",
                header { style: "pointer-events:auto;padding:14px 16px 8px;background:linear-gradient(180deg,rgba(5,7,12,.82),transparent);",
                    p { style: "margin:0;font-size:.62rem;letter-spacing:.08em;text-transform:uppercase;color:#00d2ff;font-weight:800;",
                        "Manifold · {active.id()}"
                    }
                    h2 { style: "margin:4px 0 0;font-size:1.05rem;", "{active.title()}" }
                    p { style: "margin:4px 0 0;color:#94a3b8;font-size:.74rem;", "{active.blurb()}" }
                }
                div { style: "pointer-events:auto;overflow-y:auto;padding:8px 16px 16px;display:grid;gap:10px;align-content:start;max-width:560px;",
                    if here.is_empty() {
                        p { style: "margin:0;color:#94a3b8;font-size:.78rem;background:rgba(7,9,14,.62);border:1px dashed #1a2230;border-radius:10px;padding:10px 12px;",
                            "Empty desk. Open a toolbox and place a container on this surface."
                        }
                    } else {
                        for item in here {
                            ContainerCard { key: "{item.id}", item: item }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ManifoldPager(active: ManifoldId, on_pick: EventHandler<ManifoldId>) -> Element {
    rsx! {
        nav {
            aria_label: "Manifolds",
            style: "display:flex;flex-wrap:wrap;gap:6px;align-items:center;",
            span { style: "font-size:.62rem;letter-spacing:.08em;text-transform:uppercase;color:#64748b;", "Surfaces" }
            for m in ManifoldId::ALL {
                button {
                    key: "{m.id()}",
                    r#type: "button",
                    aria_pressed: active == m,
                    style: if active == m { page_on() } else { page_off() },
                    onclick: move |_| on_pick.call(m),
                    "{m.title()}"
                }
            }
        }
    }
}

fn page_on() -> &'static str {
    "border:1px solid rgba(0,210,255,.5);background:rgba(0,210,255,.12);color:#00d2ff;border-radius:999px;padding:5px 12px;font-size:.74rem;font-weight:700;cursor:pointer;"
}

fn page_off() -> &'static str {
    "border:1px solid #1a2230;background:#131822;color:#94a3b8;border-radius:999px;padding:5px 12px;font-size:.74rem;cursor:pointer;"
}
