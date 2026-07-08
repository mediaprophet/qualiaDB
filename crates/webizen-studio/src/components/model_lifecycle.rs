use dioxus::prelude::*;
use crate::commands::invoke;

#[component]
pub fn ModelLifecycle() -> Element {
    let mut step = use_signal(|| 2);
    let mut lifecycle_label_state = use_signal(|| "StreamingVRAM".to_string());

    use_future(move || async move {
        loop {
            if let Ok(response) = invoke("wellfair_get_model_lifecycle_status", serde_wasm_bindgen::to_value(&()).unwrap()).await {
                if let Some(state_str) = response.as_string() {
                    lifecycle_label_state.set(state_str.clone());
                    let new_step = match state_str.as_str() {
                        "Discovered" => 0,
                        "MappedToDisk" => 1,
                        "StreamingVRAM" => 2,
                        "Active" => 3,
                        "Scrubbing" => 4,
                        _ => 0,
                    };
                    step.set(new_step);
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    });

    rsx! {
        div { style: "padding: 2rem; background: #f8fafc; color: #0f172a; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h1 { style: "font-size: 2rem; margin-bottom: 2rem; color: #334155;", "GGUF Model Lifecycle Orchestrator" }

            div { style: "display: flex; justify-content: space-between; position: relative; margin-bottom: 3rem;",
                div { style: "position: absolute; top: 50%; left: 0; right: 0; height: 4px; background: #cbd5e1; z-index: 1;" }
                div { style: "position: absolute; top: 50%; left: 0; width: 50%; height: 4px; background: #3b82f6; z-index: 2; transition: width 0.5s;" }

                for (i, label) in ["Discovered", "MappedToDisk", "StreamingVRAM", "Active", "Scrubbing"].iter().enumerate() {
                    div { style: "position: relative; z-index: 3; display: flex; flex-direction: column; align-items: center; gap: 8px;",
                        div { style: "width: 32px; height: 32px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-weight: bold; background: #e2e8f0; color: #64748b; border: 4px solid #f8fafc;",
                            "{i + 1}"
                        }
                        span { style: "font-size: 0.875rem; font-weight: 600; color: #94a3b8;", "{label}" }
                    }
                }
            }

            div { style: "background: #fff; padding: 24px; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.05); border: 1px solid #e2e8f0;",
                h3 { style: "margin-top: 0; font-size: 1.25rem;", "Current Phase: {lifecycle_label_state()}" }
                p { style: "color: #64748b; line-height: 1.5;", "The model weights are currently being asynchronously mapped to DirectML/wgpu buffers. The SLG Arena pointer map is active." }

                div { style: "margin: 24px 0; background: #f1f5f9; height: 24px; border-radius: 12px; overflow: hidden;",
                    div { style: "background: linear-gradient(90deg, #3b82f6, #60a5fa); width: 68%; height: 100%; display: flex; align-items: center; justify-content: center; color: white; font-size: 12px; font-weight: bold;",
                        "68% (12.4 / 18.2 GB)"
                    }
                }

                div { style: "display: flex; gap: 12px;",
                    button { style: "padding: 10px 20px; background: #ef4444; color: white; border: none; border-radius: 6px; cursor: pointer; font-weight: bold;", "Abort Loading" }
                    button { 
                        style: "padding: 10px 20px; background: #10b981; color: white; border: none; border-radius: 6px; cursor: pointer; font-weight: bold;", 
                        onclick: move |_| { 
                            let phase = if step() < 4 { step() + 1 } else { 0 };
                            let args = serde_wasm_bindgen::to_value(&serde_json::json!({ "phase": phase })).unwrap();
                            let _ = invoke("wellfair_force_model_lifecycle_phase", args);
                        }, 
                        "Force Next Phase" 
                    }
                }
            }
        }
    }
}
