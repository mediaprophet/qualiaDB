use dioxus::prelude::*;

#[component]
pub fn StatisticalAnalysis() -> Element {
    let mut data = use_signal(|| "1.2, 2.3, 3.4, 4.5, 5.6".to_string());

    rsx! {
        div {
            style: "padding: 20px; background: #1e1e2e; color: #cdd6f4; border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%;",
            h2 { style: "margin: 0; color: #fab387; border-bottom: 1px solid #313244; padding-bottom: 8px;", "Statistical Analysis" }
            div {
                label { "Data Points (comma separated)" }
                textarea {
                    value: "{data}",
                    oninput: move |e| data.set(e.value().clone()),
                    style: "width: 100%; height: 60px; padding: 8px; background: #181825; border: 1px solid #45475a; color: #cdd6f4; border-radius: 4px; margin-top: 4px; font-family: monospace;"
                }
            }
            div {
                style: "display: grid; grid-template-columns: repeat(3, 1fr); gap: 16px;",
                div {
                    style: "background: #11111b; padding: 12px; border-radius: 8px; text-align: center; border-bottom: 3px solid #fab387;",
                    div { style: "font-size: 12px; color: #a6adc8;", "Mean" }
                    div { style: "font-size: 20px; font-weight: bold;", "3.40" }
                }
                div {
                    style: "background: #11111b; padding: 12px; border-radius: 8px; text-align: center; border-bottom: 3px solid #f38ba8;",
                    div { style: "font-size: 12px; color: #a6adc8;", "Median" }
                    div { style: "font-size: 20px; font-weight: bold;", "3.40" }
                }
                div {
                    style: "background: #11111b; padding: 12px; border-radius: 8px; text-align: center; border-bottom: 3px solid #f9e2af;",
                    div { style: "font-size: 12px; color: #a6adc8;", "Std Dev" }
                    div { style: "font-size: 20px; font-weight: bold;", "1.74" }
                }
            }
            div {
                style: "flex: 1; border: 1px solid #313244; border-radius: 8px; display: flex; align-items: center; justify-content: center; background: #181825;",
                "Histogram / Box Plot View"
            }
        }
    }
}
