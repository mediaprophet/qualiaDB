use dioxus::prelude::*;

#[component]
pub fn InferenceMonitor() -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; height: 100%; background: #000; color: #0f0; font-family: monospace; padding: 16px; box-sizing: border-box;",
            div { style: "display: flex; justify-content: space-between; border-bottom: 1px solid #0f0; padding-bottom: 8px; margin-bottom: 16px;",
                span { style: "font-size: 1.2rem; font-weight: bold;", "INFERENCE STREAM MONITOR [tty1]" }
                span { "STATUS: REALTIME_ACTIVE" }
            }

            div { style: "display: flex; gap: 16px; margin-bottom: 16px;",
                div { style: "flex: 1; border: 1px solid #0f0; padding: 12px;",
                    div { "TOKENS / SEC" }
                    div { style: "font-size: 2rem;", "142.8" }
                }
                div { style: "flex: 1; border: 1px solid #0f0; padding: 12px;",
                    div { "KV CACHE UTILIZATION" }
                    div { style: "font-size: 2rem;", "64%" }
                }
                div { style: "flex: 1; border: 1px solid #0f0; padding: 12px;",
                    div { "ACTIVE BATCH SIZE" }
                    div { style: "font-size: 2rem;", "8" }
                }
            }

            div { style: "flex: 1; border: 1px solid #0f0; overflow-y: auto; padding: 12px; display: flex; flex-direction: column; gap: 4px;",
                div { "> [TRACE] Fused attention block dispatch complete (1.2ms)" }
                div { "> [TRACE] Sampling token id: 4892 (prob: 0.88)" }
                div { "> [TRACE] KV slot allocated at index 8432" }
                div { "> [INFO] Emitted: ' therefore, the hypothesis'" }
                div { "> [TRACE] Context sliding window advanced by 1" }
                div { style: "color: #aa0;", "> [WARN] Temperature scaling anomalous for token id: 11" }
                div { "> [INFO] Emitted: ' holds true.'" }
                div { "> _" }
            }
        }
    }
}
