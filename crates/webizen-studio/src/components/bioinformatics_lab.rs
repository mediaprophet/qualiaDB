use dioxus::prelude::*;

#[component]
pub fn BioinformaticsLab() -> Element {
    let mut sequence = use_signal(|| "ATCGATCGTACG".to_string());

    rsx! {
        div {
            style: "padding: 20px; background: #1e1e2e; color: #cdd6f4; border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%;",
            h2 { style: "margin: 0; color: #a6e3a1; border-bottom: 1px solid #313244; padding-bottom: 8px;", "Bioinformatics Lab" }
            div {
                label { "DNA Sequence" }
                textarea {
                    value: "{sequence}",
                    oninput: move |e| sequence.set(e.value().clone()),
                    style: "width: 100%; height: 80px; padding: 8px; background: #181825; border: 1px solid #45475a; color: #a6e3a1; border-radius: 4px; margin-top: 4px; font-family: monospace; letter-spacing: 2px;"
                }
            }
            div {
                style: "display: flex; gap: 8px;",
                button { style: "background: #313244; color: #cdd6f4; border: none; padding: 6px 12px; border-radius: 4px; cursor: pointer;", "Reverse Complement" }
                button { style: "background: #313244; color: #cdd6f4; border: none; padding: 6px 12px; border-radius: 4px; cursor: pointer;", "Translate to Protein" }
                button { style: "background: #313244; color: #cdd6f4; border: none; padding: 6px 12px; border-radius: 4px; cursor: pointer;", "GC Content" }
            }
            div {
                style: "flex: 1; background: #11111b; padding: 16px; border-radius: 8px; border: 1px solid #313244;",
                h4 { style: "margin: 0 0 8px 0; color: #bac2de;", "Analysis Output" }
                div {
                    "GC Content: 50.0%"
                    br {}
                    "Length: {sequence().len()} bp"
                }
            }
        }
    }
}
