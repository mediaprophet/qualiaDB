//! Media codecs body — Multi-domain media codecs matrix & 3-tier sandboxing.

use crate::components::poet::engine::{self, PoetEvalResult};
use dioxus::prelude::*;

const CODEC_MATRIX: &str = r#"requires [ capability("capability.invoke") ];
effect fn list_codecs() {
    return capability.invoke("Codecs.matrix", {});
}
"#;

#[component]
pub fn CodecsBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);

    rsx! {
        div { style: "display:grid;gap:8px;padding:8px;",
            p { style: muted(), "Media Codecs & Ingestion Matrix: 3-Tier sandboxing (Pure Rust/WASM, GPU neural, sidecars) with 8MB streaming chunk boundaries." }
            div { style: "display:flex;gap:8px;flex-wrap:wrap;",
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match engine::eval(CODEC_MATRIX.into(), false, Some("list_codecs".into())).await {
                                Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                                Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                                Err(e) => out.set(e),
                            }
                            busy.set(false);
                        });
                    },
                    "Codecs.matrix"
                }
            }
            if !out().is_empty() {
                pre { style: "font-size:.72rem;white-space:pre-wrap;margin:0;font-family:var(--font-mono);color:#f97316;", "{out}" }
            }
        }
    }
}

fn muted() -> &'static str {
    "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;"
}
