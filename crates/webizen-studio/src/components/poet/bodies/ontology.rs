//! Aura / SHACL catalog body.

use crate::components::poet::engine::{self, PoetEvalResult};
use dioxus::prelude::*;

const EXT: &str = r#"requires [ capability("capability.invoke") ];
effect fn ext() {
    return capability.invoke("SHACL.extensions", null);
}
"#;

const ASK: &str = r#"requires [ capability("capability.invoke") ];
effect fn ask() {
    return capability.invoke("GraphDatabase.sparql", { query: "ASK { ?s ?p ?o }", take: 1 });
}
"#;

#[component]
pub fn OntologyBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);
    rsx! {
        div { style: "display:grid;gap:8px;",
            p { style: muted(), "Aura catalog via SHACL.extensions. Full shape-IRI registry is later." }
            button {
                disabled: busy(),
                style: crate::components::poet::vibe_console::secondary(),
                onclick: move |_| {
                    busy.set(true);
                    spawn(async move {
                        match engine::eval(EXT.into(), false, Some("ext".into())).await {
                            Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                            Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                            Err(e) => out.set(e),
                        }
                        busy.set(false);
                    });
                },
                "List SHACL extensions"
            }
            button {
                disabled: busy(),
                style: crate::components::poet::vibe_console::secondary(),
                onclick: move |_| {
                    busy.set(true);
                    spawn(async move {
                        match engine::eval(ASK.into(), false, Some("ask".into())).await {
                            Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                            Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                            Err(e) => out.set(e),
                        }
                        busy.set(false);
                    });
                },
                "SPARQL ASK live snapshot"
            }
            if !out().is_empty() {
                pre { style: "font-size:.76rem;white-space:pre-wrap;margin:0;max-height:180px;overflow:auto;", "{out}" }
            }
        }
    }
}

fn muted() -> &'static str {
    "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;"
}
