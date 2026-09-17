//! Cooperative economics body — SDN permissive lanes & true-cost personal modeler.

use crate::components::poet::engine::{self, PoetEvalResult};
use dioxus::prelude::*;

const ECON_RATE: &str = r#"requires [ capability("capability.invoke") ];
effect fn peer_rate() {
    return capability.invoke("Econ.evaluate_peer", {
        peer: "did:qualia:0x3f8a...",
        entity_type: "human_commons"
    });
}
"#;

#[component]
pub fn EconomicsBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);

    rsx! {
        div { style: "display:grid;gap:8px;padding:8px;",
            p { style: muted(), "Socially Defined Networking & Ontological Economics: 25GB human commons quota, true-cost hardware/power calculator." }
            div { style: "display:flex;gap:8px;flex-wrap:wrap;",
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match engine::eval(ECON_RATE.into(), false, Some("peer_rate".into())).await {
                                Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                                Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                                Err(e) => out.set(e),
                            }
                            busy.set(false);
                        });
                    },
                    "Econ.evaluate_peer"
                }
            }
            if !out().is_empty() {
                pre { style: "font-size:.72rem;white-space:pre-wrap;margin:0;font-family:var(--font-mono);color:#34d399;", "{out}" }
            }
        }
    }
}

fn muted() -> &'static str {
    "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;"
}
