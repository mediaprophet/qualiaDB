//! Solid body — W3C Solid Pod LDP explorer & zero-lock-in migration wizard.

use crate::components::poet::engine::{self, PoetEvalResult};
use dioxus::prelude::*;

const SOLID_INBOX: &str = r#"requires [ capability("capability.invoke") ];
effect fn solid_inbox() {
    return capability.invoke("Solid.list_container", {
        url: "https://pod.thorne.id/inbox/"
    });
}
"#;

#[component]
pub fn SolidBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);

    rsx! {
        div { style: "display:grid;gap:8px;padding:8px;",
            p { style: muted(), "W3C Solid LDP Pod Browser & Zero-Lock-In Migration Wizard. Super-Quins to Turtle, CML to RDFa." }
            div { style: "display:flex;gap:8px;flex-wrap:wrap;",
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match engine::eval(SOLID_INBOX.into(), false, Some("solid_inbox".into())).await {
                                Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                                Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                                Err(e) => out.set(e),
                            }
                            busy.set(false);
                        });
                    },
                    "Solid.list_container"
                }
            }
            if !out().is_empty() {
                pre { style: "font-size:.72rem;white-space:pre-wrap;margin:0;font-family:var(--font-mono);color:#38bdf8;", "{out}" }
            }
        }
    }
}

fn muted() -> &'static str {
    "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;"
}
