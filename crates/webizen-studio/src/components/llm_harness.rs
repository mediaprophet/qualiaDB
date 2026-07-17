#![allow(non_snake_case)]
use crate::components::honesty_chip::{HonestyChip, HonestyLevel};
use crate::components::shoelace::*;
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// The engine runs NATIVELY; this wasm UI only reads its telemetry over the Tauri invoke bridge.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn invoke(cmd: &str, args: wasm_bindgen::JsValue) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[component]
pub fn LlmHarness() -> Element {
    let mut tokens_per_sec = use_signal(|| 0.0);
    let mut tok_source = use_signal(|| "none".to_string());
    let mut vram_usage_gb = use_signal(|| 0.0);
    let mut vram_total_gb = use_signal(|| 0.0);
    let mut loaded_model = use_signal(|| String::from("none"));
    let mut lifecycle = use_signal(|| String::from("—"));
    let mut thermal = use_signal(|| String::from("—"));
    let mut backend = use_signal(|| String::from("—"));
    let mut ollama_note = use_signal(|| Option::<String>::None);

    use_future(move || async move {
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (
            &mut tokens_per_sec,
            &mut tok_source,
            &mut vram_usage_gb,
            &mut vram_total_gb,
            &mut loaded_model,
            &mut lifecycle,
            &mut thermal,
            &mut backend,
            &mut ollama_note,
        );
        #[cfg(target_arch = "wasm32")]
        loop {
            if let Ok(response) =
                invoke("wellfair_get_llm_telemetry", serde_wasm_bindgen::to_value(&()).unwrap()).await
            {
                if let Ok(telemetry) = serde_wasm_bindgen::from_value::<serde_json::Value>(response) {
                    if let Some(tps) = telemetry["tokens_per_sec"].as_f64() {
                        tokens_per_sec.set(tps);
                    }
                    if let Some(s) = telemetry["tokens_per_sec_source"].as_str() {
                        tok_source.set(s.to_string());
                    }
                    if let Some(vu) = telemetry["vram_usage_gb"].as_f64() {
                        vram_usage_gb.set(vu);
                    }
                    if let Some(vt) = telemetry["vram_total_gb"].as_f64() {
                        vram_total_gb.set(vt);
                    }
                    if let Some(model) = telemetry["loaded_model"].as_str() {
                        loaded_model.set(model.to_string());
                    }
                    if let Some(lc) = telemetry["model_lifecycle"].as_str() {
                        lifecycle.set(lc.to_string());
                    }
                    if let Some(th) = telemetry["thermal_state"].as_str() {
                        thermal.set(th.to_string());
                    }
                    if let Some(b) = telemetry["inference_backend"].as_str() {
                        backend.set(b.to_string());
                    }
                    ollama_note.set(
                        telemetry["ollama_optional_note"]
                            .as_str()
                            .map(|s| s.to_string()),
                    );
                }
            }
            gloo_timers::future::TimeoutFuture::new(500).await;
        }
    });

    let model_ready = {
        let m = loaded_model();
        !m.is_empty() && m != "none"
    };
    let tok_label = if tok_source() == "none" {
        "Tokens/sec (not measured yet)".to_string()
    } else {
        format!("Tokens/sec (last turn · {})", tok_source())
    };

    rsx! {
        div { class: "llm-harness-pane flex flex-col h-full",
            SlCard {
                div { slot: "header",
                    div { style: "display:flex; flex-wrap:wrap; gap:8px; align-items:center;",
                        span { "LLM Local Engine Harness" }
                        if model_ready {
                            HonestyChip {
                                level: HonestyLevel::Partial,
                                detail: "Live probe — not a mock HUD".to_string(),
                            }
                        } else {
                            HonestyChip {
                                level: HonestyLevel::NeedsModel,
                                detail: "No active GGUF".to_string(),
                            }
                        }
                    }
                }
                div { class: "flex-col flex gap-4",
                    div {
                        h4 { "Backend" }
                        p { style: "margin:0; font-size:13px; color:#94a3b8;", "{backend}" }
                        if let Some(note) = ollama_note() {
                            p { style: "margin:6px 0 0; font-size:12px; color:#fde68a;", "{note}" }
                        }
                    }
                    div {
                        h4 { "Loaded model" }
                        p { style: "margin:0; font-size:13px;", "{loaded_model}" }
                        p { style: "margin:4px 0 0; font-size:12px; color:#94a3b8;",
                            "Lifecycle: {lifecycle} · Thermal: {thermal}"
                        }
                    }
                    div {
                        h4 { "Telemetry HUD" }
                        div { class: "grid grid-cols-2 gap-2",
                            div { class: "stat-box",
                                "{tok_label}: {tokens_per_sec():.2}"
                            }
                            div { class: "stat-box",
                                "VRAM: {vram_usage_gb():.2} / {vram_total_gb():.2} GB"
                            }
                        }
                        p { style: "margin:8px 0 0; font-size:11px; color:#64748b;",
                            "0 tok/s with source “none” means no completed turn this process — not a fake 18.3."
                        }
                    }
                }
            }
        }
    }
}
