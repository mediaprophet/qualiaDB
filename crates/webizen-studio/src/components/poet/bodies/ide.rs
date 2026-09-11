//! IDE body — 6-zone VibeScript integrated development environment.

use crate::components::poet::engine::{self, PoetEvalResult};
use crate::components::poet::lexicon_bay::LexiconBay;
use dioxus::prelude::*;

const VIBE_REPL: &str = r#"requires [ capability("capability.invoke") ];
effect fn repl_demo() {
    return capability.invoke("Vibe.eval", {
        script: "let x = 40 + 2; return x;"
    });
}
"#;

#[component]
pub fn IdeBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);

    let mut bay = use_signal(|| "catalog");

    rsx! {
        div { style: "display:grid;gap:8px;padding:8px;",
            div {
                class: "lexicon-peer-tabs",
                role: "tablist",
                "aria-label": "Vibe REPL Problems Catalog · Lexicon",
                button {
                    r#type: "button",
                    class: if bay() == "repl" { "lexicon-peer-tab is-active" } else { "lexicon-peer-tab" },
                    "data-bay-tab": "repl",
                    onclick: move |_| bay.set("repl"),
                    "Vibe REPL"
                }
                button {
                    r#type: "button",
                    class: if bay() == "problems" { "lexicon-peer-tab is-active" } else { "lexicon-peer-tab" },
                    "data-bay-tab": "problems",
                    onclick: move |_| bay.set("problems"),
                    "Problems"
                }
                button {
                    r#type: "button",
                    class: if bay() == "catalog" { "lexicon-peer-tab is-active" } else { "lexicon-peer-tab" },
                    "data-bay-tab": "catalog",
                    onclick: move |_| bay.set("catalog"),
                    "Catalog · Lexicon"
                }
            }
            if bay() == "catalog" {
                LexiconBay {}
            } else if bay() == "problems" {
                p { style: muted(), "held / not yet — diagnose appears here after a run." }
            } else {
            p { style: muted(), "6-Zone Modular Poet IDE: Multi-tab code editor, interactive VibeScript REPL, symbol explorer, and real-time gas tracking." }
            div { style: "display:flex;gap:8px;flex-wrap:wrap;",
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match engine::eval(VIBE_REPL.into(), false, Some("repl_demo".into())).await {
                                Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                                Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                                Err(e) => out.set(e),
                            }
                            busy.set(false);
                        });
                    },
                    "Vibe.eval"
                }
            }
            if !out().is_empty() {
                pre { style: "font-size:.72rem;white-space:pre-wrap;margin:0;font-family:var(--font-mono);color:#38bdf8;", "{out}" }
            }
            }
        }
    }
}

fn muted() -> &'static str {
    "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;"
}
