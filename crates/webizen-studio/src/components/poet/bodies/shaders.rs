//! Shaders body — WGSL Forge shader pipelines & WebGPU compute acceleration.

use crate::components::poet::engine::{self, PoetEvalResult};
use dioxus::prelude::*;

const SHADER_LIST: &str = r#"requires [ capability("capability.invoke") ];
effect fn list_shaders() {
    return capability.invoke("Shaders.list", {});
}
"#;

const WGSL_GENERATE: &str = r#"requires [ capability("capability.invoke") ];
effect fn gen_shader() {
    return capability.invoke("WgslForge.generate", { kernel: "affine-f32", workgroup_size: 64 });
}
"#;

const WGSL_ROOFLINE: &str = r#"requires [ capability("capability.invoke") ];
effect fn roofline() {
    return capability.invoke("WgslForge.roofline", { kernel: "gemm-tiled-f32", elements: 65536 });
}
"#;

const WGSL_ATTENTION: &str = r#"requires [ capability("capability.invoke") ];
effect fn synth_attn() {
    return capability.invoke("WgslForge.attention_synthesize", { seq_len: 128, head_dim: 64 });
}
"#;

#[component]
/// Dioxus `ShadersBody` component.
pub fn ShadersBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);

    let mut run_op = move |src: &'static str, fn_name: &'static str| {
        busy.set(true);
        spawn(async move {
            match engine::eval(src.into(), false, Some(fn_name.into())).await {
                Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                Err(e) => out.set(e),
            }
            busy.set(false);
        });
    };

    rsx! {
        div { style: "display:grid;gap:8px;padding:8px;",
            p { style: muted(), "WGSL Forge: Deterministic compute & viewport shader synthesis, Naga validation, Roofline analysis, and Causal Attention WGSL generation." }
            div { style: "display:flex;gap:8px;flex-wrap:wrap;",
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| run_op(SHADER_LIST, "list_shaders"),
                    "Shaders.list"
                }
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| run_op(WGSL_GENERATE, "gen_shader"),
                    "WgslForge.generate"
                }
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| run_op(WGSL_ROOFLINE, "roofline"),
                    "WgslForge.roofline"
                }
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| run_op(WGSL_ATTENTION, "synth_attn"),
                    "WgslForge.attention_synthesize"
                }
            }
            if !out().is_empty() {
                pre { style: "font-size:.72rem;white-space:pre-wrap;margin:0;max-height:180px;overflow:auto;font-family:var(--font-mono);color:#ec4899;", "{out}" }
            }
        }
    }
}

fn muted() -> &'static str {
    "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;"
}
