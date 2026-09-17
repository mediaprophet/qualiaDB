//! Shaders body — WGSL Forge shader pipelines & WebGPU compute acceleration.

use crate::components::poet::engine::{self, PoetEvalResult};
use dioxus::prelude::*;

const SHADER_LIST: &str = r#"requires [ capability("capability.invoke") ];
effect fn list_shaders() {
    return capability.invoke("Shaders.list", {});
}
"#;

#[component]
pub fn ShadersBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);

    rsx! {
        div { style: "display:grid;gap:8px;padding:8px;",
            p { style: muted(), "WGSL Forge: 8 Specialized GPU shader pipelines (CyberGlass, Barnes-Hut Graph Physics, Wire Particles, 10D Manifold Grid)." }
            div { style: "display:flex;gap:8px;flex-wrap:wrap;",
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match engine::eval(SHADER_LIST.into(), false, Some("list_shaders".into())).await {
                                Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                                Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                                Err(e) => out.set(e),
                            }
                            busy.set(false);
                        });
                    },
                    "Shaders.list"
                }
            }
            if !out().is_empty() {
                pre { style: "font-size:.72rem;white-space:pre-wrap;margin:0;font-family:var(--font-mono);color:#ec4899;", "{out}" }
            }
        }
    }
}

fn muted() -> &'static str {
    "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;"
}
