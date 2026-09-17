//! Domain presence body — Inalienable web publishing, WebID card & BIND zone exporter.

use crate::components::poet::engine::{self, PoetEvalResult};
use dioxus::prelude::*;

const DOMAIN_STATUS: &str = r#"requires [ capability("capability.invoke") ];
effect fn domain_status() {
    return capability.invoke("Domain.info", {
        domain: "thorne.id"
    });
}
"#;

#[component]
pub fn DomainsBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);

    rsx! {
        div { style: "display:grid;gap:8px;padding:8px;",
            p { style: muted(), "Domain & Web Presence: 4 Pillars (Web publishing, purpose-bound mail, Solid pod, DNS tunnels) and BIND zone generation." }
            div { style: "display:flex;gap:8px;flex-wrap:wrap;",
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match engine::eval(DOMAIN_STATUS.into(), false, Some("domain_status".into())).await {
                                Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                                Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                                Err(e) => out.set(e),
                            }
                            busy.set(false);
                        });
                    },
                    "Domain.info"
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
