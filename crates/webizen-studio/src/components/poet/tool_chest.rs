//! Tool chest: the dock. Toolboxes are drawers. Tools place containers.

use super::kinds::{ContainerInstance, ManifoldId, ToolSpec, ToolboxId};
use dioxus::prelude::*;

#[component]
pub fn ToolChest(
    open: ToolboxId,
    on_open: EventHandler<ToolboxId>,
    on_tool: EventHandler<ToolSpec>,
) -> Element {
    rsx! {
        aside {
            style: "background:#0b0e14;border:1px solid #1a2230;border-radius:12px;padding:10px;display:grid;gap:8px;align-content:start;",
            p { style: "margin:0;font-size:.62rem;letter-spacing:.08em;text-transform:uppercase;color:#00d2ff;font-weight:800;",
                "Tool chest"
            }
            p { style: "margin:0;font-size:.7rem;color:#64748b;", "Drawers are toolboxes. A tool places a container." }
            div { style: "display:flex;flex-wrap:wrap;gap:4px;",
                for tb in ToolboxId::ALL {
                    button {
                        key: "{tb.id()}",
                        r#type: "button",
                        aria_pressed: open == tb,
                        style: if open == tb { chest_on() } else { chest_off() },
                        onclick: move |_| on_open.call(tb),
                        "{tb.title()}"
                    }
                }
            }
            div { style: "display:grid;gap:6px;",
                p { style: "margin:0;font-size:.78rem;color:#e8eef7;font-weight:700;", "{open.title()} toolbox" }
                for (i, tool) in open.tools().iter().enumerate() {
                    button {
                        key: "{i}",
                        r#type: "button",
                        style: "text-align:left;border:1px solid #1a2230;background:#131822;color:#e8eef7;border-radius:8px;padding:7px 10px;font-size:.76rem;cursor:pointer;",
                        onclick: move |_| on_tool.call(*tool),
                        "{tool.label}"
                    }
                }
            }
        }
    }
}

pub fn place(
    next_id: &mut u32,
    on: ManifoldId,
    spec: ToolSpec,
    into: &mut Vec<ContainerInstance>,
) {
    if let Some(kind) = spec.places {
        into.push(ContainerInstance {
            id: *next_id,
            kind,
            on,
        });
        *next_id += 1;
    }
}

fn chest_on() -> &'static str {
    "border:1px solid rgba(0,210,255,.5);background:rgba(0,210,255,.12);color:#00d2ff;border-radius:8px;padding:5px 8px;font-size:.72rem;cursor:pointer;"
}

fn chest_off() -> &'static str {
    "border:1px solid #1a2230;background:#131822;color:#94a3b8;border-radius:8px;padding:5px 8px;font-size:.72rem;cursor:pointer;"
}
