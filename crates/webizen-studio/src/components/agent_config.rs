use dioxus::prelude::*;

#[component]
pub fn AgentConfig() -> Element {
    let mut temperature = use_signal(|| 0.7f64);
    let mut top_p = use_signal(|| 0.9f64);

    rsx! {
        div { style: "padding: 24px; background: #111827; color: #f3f4f6; height: 100%; box-sizing: border-box; overflow-y: auto;",
            div { style: "max-width: 800px; margin: 0 auto;",
                h2 { style: "color: #a78bfa; margin-bottom: 24px; font-size: 28px; border-bottom: 1px solid #374151; padding-bottom: 12px;", "Autonomous Agent Configuration" }

                div { style: "background: #1f2937; padding: 20px; border-radius: 12px; margin-bottom: 20px; box-shadow: 0 4px 6px rgba(0,0,0,0.3);",
                    h3 { style: "margin-top: 0; color: #e5e7eb;", "Inference Hyperparameters" }
                    div { style: "margin-bottom: 16px;",
                        label { style: "display: block; margin-bottom: 8px; color: #9ca3af;", "Temperature ({temperature()})" }
                        input {
                            type: "range", min: "0.0", max: "2.0", step: "0.05", value: "{temperature}",
                            style: "width: 100%; accent-color: #8b5cf6;",
                            oninput: move |e| if let Ok(v) = e.value().parse() { temperature.set(v); }
                        }
                    }
                    div { style: "margin-bottom: 16px;",
                        label { style: "display: block; margin-bottom: 8px; color: #9ca3af;", "Top-P (Nucleus Sampling) ({top_p()})" }
                        input {
                            type: "range", min: "0.0", max: "1.0", step: "0.05", value: "{top_p}",
                            style: "width: 100%; accent-color: #8b5cf6;",
                            oninput: move |e| if let Ok(v) = e.value().parse() { top_p.set(v); }
                        }
                    }
                }

                div { style: "background: #1f2937; padding: 20px; border-radius: 12px; margin-bottom: 20px; box-shadow: 0 4px 6px rgba(0,0,0,0.3);",
                    h3 { style: "margin-top: 0; color: #e5e7eb;", "Behavioral Bounds (Deontic Constraints)" }
                    div { style: "display: flex; flex-direction: column; gap: 12px;",
                        label { style: "display: flex; align-items: center; gap: 12px;",
                            input { type: "checkbox", checked: true, style: "width: 18px; height: 18px; accent-color: #8b5cf6;" }
                            span { "Enforce Non-Adversarial Fallback" }
                        }
                        label { style: "display: flex; align-items: center; gap: 12px;",
                            input { type: "checkbox", checked: true, style: "width: 18px; height: 18px; accent-color: #8b5cf6;" }
                            span { "Halt on Epistemic Contradiction (> 0.8 certainty)" }
                        }
                        label { style: "display: flex; align-items: center; gap: 12px;",
                            input { type: "checkbox", style: "width: 18px; height: 18px; accent-color: #8b5cf6;" }
                            span { "Allow External Web Search (MCP)" }
                        }
                    }
                }

                button { style: "background: #8b5cf6; color: white; padding: 12px 24px; border: none; border-radius: 8px; font-weight: bold; cursor: pointer; width: 100%; transition: background 0.2s;",
                    "Apply Agent Configuration"
                }
            }
        }
    }
}
