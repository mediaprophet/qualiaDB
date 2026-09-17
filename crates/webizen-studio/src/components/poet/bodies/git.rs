//! Git body — In-process Git forge, P2P remotes & DID-signed commits.

use crate::components::poet::engine::{self, PoetEvalResult};
use dioxus::prelude::*;

const GIT_STATUS: &str = r#"requires [ capability("capability.invoke") ];
effect fn git_status() {
    return capability.invoke("Git.status", {
        repo: "qualia-27062026",
        branch: "0.0.34"
    });
}
"#;

#[component]
pub fn GitBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);

    rsx! {
        div { style: "display:grid;gap:8px;padding:8px;",
            p { style: muted(), "Distributed Git Forge: P2P remotes (Domain SSH, WebRTC Mesh, Solid Pod) with Ed25519 DID commit signatures." }
            div { style: "display:flex;gap:8px;flex-wrap:wrap;",
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match engine::eval(GIT_STATUS.into(), false, Some("git_status".into())).await {
                                Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                                Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                                Err(e) => out.set(e),
                            }
                            busy.set(false);
                        });
                    },
                    "Git.status"
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
