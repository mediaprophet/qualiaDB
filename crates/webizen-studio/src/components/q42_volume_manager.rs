use dioxus::prelude::*;

#[component]
pub fn Q42VolumeManager() -> Element {
    rsx! {
        div { style: "padding: 20px; background: #e0e5ec; min-height: 100vh; font-family: sans-serif;",
            h2 { style: "color: #2d3748; margin-top: 0;", "Q42 Native Volume Orchestrator" }

            div { style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 20px;",
                // Volume Card
                div { style: "background: #e0e5ec; padding: 20px; border-radius: 16px; box-shadow: 9px 9px 16px rgb(163,177,198,0.6), -9px -9px 16px rgba(255,255,255, 0.5);",
                    h3 { style: "margin-top: 0; color: #4a5568;", "/dev/q42/vol0 (Primary Arena)" }
                    div { style: "margin: 16px 0; background: #d1d5db; height: 12px; border-radius: 6px; overflow: hidden;",
                        div { style: "background: #3182ce; width: 45%; height: 100%; box-shadow: inset 0 2px 4px rgba(0,0,0,0.2);" }
                    }
                    div { style: "display: flex; justify-content: space-between; font-size: 14px; color: #718096;",
                        span { "18.9 GB Used" }
                        span { "42.0 GB Total" }
                    }
                    button { style: "margin-top: 16px; width: 100%; padding: 10px; background: transparent; border: none; border-radius: 8px; box-shadow: inset 4px 4px 8px rgba(163,177,198,0.5), inset -4px -4px 8px rgba(255,255,255,0.8); cursor: pointer; color: #2b6cb0; font-weight: bold;",
                        "Expand Volume"
                    }
                }

                // Volume Card
                div { style: "background: #e0e5ec; padding: 20px; border-radius: 16px; box-shadow: 9px 9px 16px rgb(163,177,198,0.6), -9px -9px 16px rgba(255,255,255, 0.5);",
                    h3 { style: "margin-top: 0; color: #4a5568;", "/dev/q42/vol1 (Swap/GGUF)" }
                    div { style: "margin: 16px 0; background: #d1d5db; height: 12px; border-radius: 6px; overflow: hidden;",
                        div { style: "background: #e53e3e; width: 85%; height: 100%; box-shadow: inset 0 2px 4px rgba(0,0,0,0.2);" }
                    }
                    div { style: "display: flex; justify-content: space-between; font-size: 14px; color: #718096;",
                        span { style: "color: #c53030; font-weight: bold;", "110 GB Used" }
                        span { "128.0 GB Total" }
                    }
                    button { style: "margin-top: 16px; width: 100%; padding: 10px; background: transparent; border: none; border-radius: 8px; box-shadow: inset 4px 4px 8px rgba(163,177,198,0.5), inset -4px -4px 8px rgba(255,255,255,0.8); cursor: pointer; color: #c53030; font-weight: bold;",
                        "Run Scrubbing Task"
                    }
                }
            }
        }
    }
}
