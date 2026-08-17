//! Vibe console — lives inside a `code` container (or the Vibe manifold).

use super::engine::{self, PoetEvalResult};
use dioxus::prelude::*;

const SAMPLE_CELL: &str = "= math.max(0, math.min(100, 42))";
const SAMPLE_QUERY: &str = r#"requires [ capability("graph.read") ];
fn count() {
    let rows = graph.query(?s, clinic:hasCondition, ?o, take: 8)?;
    return rows;
}
"#;
const SAMPLE_INVOKE: &str = r#"requires [ capability("capability.invoke") ];
effect fn list() {
    return capability.invoke("CapabilityDiscovery.list", null);
}
"#;
const SAMPLE_SPARQL: &str = r#"requires [ capability("capability.invoke") ];
effect fn ask() {
    return capability.invoke("GraphDatabase.sparql", "ASK WHERE { ?s ?p ?o }");
}
"#;
const SAMPLE_RENDER: &str = r#"requires [ capability("capability.invoke") ];
effect fn scene() {
    return capability.invoke("Render.scene", { kind: "media" });
}
"#;

#[component]
pub fn VibeConsole() -> Element {
    let mut source = use_signal(|| SAMPLE_CELL.to_string());
    let mut function_name = use_signal(|| String::new());
    let mut result = use_signal(PoetEvalResult::default);
    let mut busy = use_signal(|| false);
    let mut status = use_signal(|| "Run a cell or function. Errors are diagnose JSON.".to_string());

    let mut run = move |as_cell: bool, fn_name: Option<String>| {
        busy.set(true);
        let src = source();
        spawn(async move {
            match engine::eval(src, as_cell, fn_name).await {
                Ok(value) => {
                    status.set(if value.ok {
                        format!(
                            "{} · {} · rev={}",
                            value.language, value.honesty, value.revision
                        )
                    } else {
                        value.diagnostic.clone().unwrap_or_else(|| "rejected".into())
                    });
                    result.set(value);
                }
                Err(error) => status.set(format!("invoke failed: {error}")),
            }
            busy.set(false);
        });
    };

    rsx! {
        div { style: "display:grid;gap:8px;",
            div { style: "display:flex;gap:6px;flex-wrap:wrap;",
                button { r#type: "button", style: chip(), onclick: move |_| { source.set(SAMPLE_CELL.into()); function_name.set(String::new()); }, "cell" }
                button { r#type: "button", style: chip(), onclick: move |_| { source.set(SAMPLE_QUERY.into()); function_name.set("count".into()); }, "graph.query" }
                button { r#type: "button", style: chip(), onclick: move |_| { source.set(SAMPLE_INVOKE.into()); function_name.set("list".into()); }, "invoke" }
                button { r#type: "button", style: chip(), onclick: move |_| { source.set(SAMPLE_SPARQL.into()); function_name.set("ask".into()); }, "SPARQL ASK" }
                button { r#type: "button", style: chip(), onclick: move |_| { source.set(SAMPLE_RENDER.into()); function_name.set("scene".into()); }, "Render.scene" }
            }
            textarea {
                style: field(),
                value: "{source}",
                oninput: move |e| source.set(e.value()),
            }
            div { style: "display:flex;gap:8px;flex-wrap:wrap;align-items:center;",
                button { disabled: busy(), style: primary(), onclick: move |_| run(true, None), "Run cell" }
                button {
                    disabled: busy(),
                    style: secondary(),
                    onclick: move |_| {
                        let name = function_name();
                        if name.is_empty() { run(false, None); } else { run(false, Some(name)); }
                    },
                    "Run function"
                }
                input {
                    style: "background:#131822;border:1px solid #1a2230;color:#e8eef7;border-radius:6px;padding:7px 10px;font-size:.78rem;width:120px;",
                    placeholder: "fn name",
                    value: "{function_name}",
                    oninput: move |e| function_name.set(e.value()),
                }
            }
            p { style: "margin:0;color:#94a3b8;font-size:.72rem;", "{status}" }
            if !result().value.is_empty() || result().diagnostic.is_some() {
                pre { style: "background:#131822;border:1px solid #1a2230;border-radius:8px;padding:10px;white-space:pre-wrap;font-size:.76rem;",
                    if result().ok { "ok {result().value}" } else { "{result().diagnostic.clone().unwrap_or_default()}" }
                }
            }
        }
    }
}

pub fn chip() -> &'static str {
    "border:1px solid #334155;background:#131822;color:#e8eef7;border-radius:999px;padding:4px 10px;font-size:.72rem;cursor:pointer;"
}

pub fn primary() -> &'static str {
    "background:#00d2ff;color:#07090e;border:none;padding:8px 14px;border-radius:6px;font-weight:700;cursor:pointer;"
}

pub fn secondary() -> &'static str {
    "background:#1a2230;color:#e8eef7;border:1px solid #334155;padding:8px 14px;border-radius:6px;cursor:pointer;"
}

fn field() -> &'static str {
    "width:100%;min-height:140px;font-family:ui-monospace,monospace;font-size:.82rem;background:#131822;color:#e8eef7;border:1px solid #1a2230;border-radius:8px;padding:10px;"
}
