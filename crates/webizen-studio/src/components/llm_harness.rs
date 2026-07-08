#![allow(non_snake_case)]
use crate::components::shoelace::*;
use dioxus::prelude::*;
use crate::commands::invoke;

#[component]
pub fn LlmHarness() -> Element {
    let mut tokens_per_sec = use_signal(|| 0.0);
    let mut vram_usage_gb = use_signal(|| 0.0);
    let mut vram_total_gb = use_signal(|| 0.0);
    let mut loaded_model = use_signal(|| String::new());

    use_future(move || async move {
        loop {
            if let Ok(response) = invoke("wellfair_get_llm_telemetry", serde_wasm_bindgen::to_value(&()).unwrap()).await {
                if let Ok(telemetry) = serde_wasm_bindgen::from_value::<serde_json::Value>(response) {
                    if let Some(tps) = telemetry["tokens_per_sec"].as_f64() { tokens_per_sec.set(tps); }
                    if let Some(vu) = telemetry["vram_usage_gb"].as_f64() { vram_usage_gb.set(vu); }
                    if let Some(vt) = telemetry["vram_total_gb"].as_f64() { vram_total_gb.set(vt); }
                    if let Some(model) = telemetry["loaded_model"].as_str() { loaded_model.set(model.to_string()); }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    });

    rsx! {
        div { class: "llm-harness-pane flex flex-col h-full",
            SlCard {
                div { slot: "header",
                    "LLM Local Engine Harness"
                }
                div { class: "flex-col flex gap-4",
                    div {
                        h4 { "Loaded GGUF Matrix" }
                        SlSelect { placeholder: "Select Quantized Model",
                            // Placeholder options
                            "Meta-Llama-3-8B-Instruct.Q4_K_M"
                            "Phi-3-mini-4k-instruct-q4"
                        }
                    }
                    div {
                        h4 { "Telemetry HUD" }
                        div { class: "grid grid-cols-2 gap-2",
                            div { class: "stat-box", "Tokens/Sec: {tokens_per_sec():.1}" }
                            div { class: "stat-box", "VRAM Usage: {vram_usage_gb():.1} / {vram_total_gb():.1} GB" }
                        }
                    }
                }
            }
        }
    }
}
