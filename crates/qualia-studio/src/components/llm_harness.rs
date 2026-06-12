#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::components::shoelace::*;

#[component]
pub fn LlmHarness() -> Element {
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
                            div { class: "stat-box", "Tokens/Sec: 0.0" }
                            div { class: "stat-box", "VRAM Usage: 0.0 GB" }
                        }
                    }
                }
            }
        }
    }
}
