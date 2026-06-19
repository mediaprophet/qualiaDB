#![allow(non_snake_case)]
use crate::components::ambient_field::{AmbientFieldCanvas, AmbientTelemetry, AMBIENT_CANVAS_ID};
use crate::components::shoelace::*;
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use serde::Deserialize;
#[cfg(target_arch = "wasm32")]
use serde_json::json;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(
        cmd: &str,
        args: js_sys::Object,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[cfg(target_arch = "wasm32")]
async fn invoke_telemetry() -> Result<AmbientTelemetry, String> {
    let js_args = serde_wasm_bindgen::to_value(&json!({})).map_err(|e| e.to_string())?;
    let value = tauri_invoke("get_system_telemetry", js_args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    serde_wasm_bindgen::from_value(value).map_err(|e| e.to_string())
}

#[component]
pub fn LlmHarness() -> Element {
    let mut telemetry = use_signal(AmbientTelemetry::default);

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        spawn(async move {
            loop {
                if let Ok(t) = invoke_telemetry().await {
                    telemetry.set(t);
                }
                gloo_timers::future::TimeoutFuture::new(500).await;
            }
        });
    });

    let t = telemetry.read().clone();
    let heat_pct = (t.llm_heat * 100.0) as u32;
    let bake_pct = (t.baking_crystallization * 100.0) as u32;
    let mem_pct = (t.memory_pressure * 100.0) as u32;

    rsx! {
        div {
            class: "llm-harness-pane",
            style: "display:flex;flex-direction:column;gap:12px;height:100%;min-height:0;",

            div {
                style: "position:relative;flex:0 0 auto;",
                AmbientFieldCanvas {
                    canvas_id: AMBIENT_CANVAS_ID.to_string(),
                    height_px: 220,
                }
                div {
                    style: "position:absolute;top:10px;left:12px;font-size:0.72rem;color:rgba(232,238,247,0.85);pointer-events:none;",
                    "Ambient epistemic field · LLM heat {heat_pct}% · ontology bake {bake_pct}%"
                }
            }

            SlCard {
                div { slot: "header", "LLM Local Engine Harness" }
                div { style: "display:flex;flex-direction:column;gap:14px;",
                    div {
                        h4 { style: "margin:0 0 6px;font-size:0.85rem;", "Loaded GGUF Matrix" }
                        SlSelect { placeholder: "Select Quantized Model",
                            "Meta-Llama-3-8B-Instruct.Q4_K_M"
                            "Phi-3-mini-4k-instruct-q4"
                        }
                    }
                    div {
                        h4 { style: "margin:0 0 6px;font-size:0.85rem;", "Stack telemetry (live)" }
                        div {
                            style: "display:grid;grid-template-columns:repeat(3,1fr);gap:8px;font-size:0.78rem;",
                            div { "LLM heat: {heat_pct}%" }
                            div { "Memory pressure: {mem_pct}%" }
                            div { "Logic flashes: {(t.logic_flashes * 100.0) as u32}%" }
                            div { "Network ripple: {(t.network_ripple * 100.0) as u32}%" }
                            div { "Ontology bake: {bake_pct}%" }
                            div { "Manifold: {(t.manifold_pressure * 100.0) as u32}%" }
                        }
                    }
                    p {
                        style: "margin:0;font-size:0.72rem;color:var(--qualia-text-muted,#8fa3bf);line-height:1.45;",
                        "Background field follows BACKGROUND_VISUALISATION.md: particles are shader/canvas-driven from ",
                        code { "SystemTelemetry" },
                        " (memory, mesh I/O, ontology jobs, GSR queries, LLM heat). Anatomy GLB ingest will add spatial anchors later."
                    }
                }
            }
        }
    }
}