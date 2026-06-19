use dioxus::prelude::*;

#[component]
pub fn ComputationalLinguisticsQapp() -> Element {
    let mut task = use_signal(|| "Parsing".to_string());
    let mut model = use_signal(|| "Transformer".to_string());
    let mut language_family = use_signal(|| "Indo-European".to_string());
    let mut accuracy = use_signal(|| 85u32);
    let mut training_data_size = use_signal(|| "1B–100B".to_string());
    let mut notes = use_signal(|| String::new());

    let tasks = [
        "Parsing",
        "Named Entity Recognition",
        "Sentiment Analysis",
        "Machine Translation",
        "Coreference Resolution",
        "Speech Recognition",
        "Text Generation",
    ];
    let models = ["Rule-based", "Statistical", "Neural", "Transformer", "LLM"];
    let language_families = [
        "Indo-European",
        "Sino-Tibetan",
        "Afro-Asiatic",
        "Niger-Congo",
        "Dravidian",
        "Austronesian",
        "Other",
    ];
    let training_sizes = ["<1M", "1M–1B", "1B–100B", "100B+"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 {
                style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;",
                "Computational Linguistics"
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Task" }
                select {
                    value: "{task}",
                    onchange: move |e| task.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in tasks { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Model" }
                select {
                    value: "{model}",
                    onchange: move |e| model.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in models { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Language Family" }
                select {
                    value: "{language_family}",
                    onchange: move |e| language_family.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in language_families { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Accuracy: {accuracy}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{accuracy}",
                    oninput: move |e| accuracy.set(e.value().parse().unwrap_or(85)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Training Data Size (tokens)" }
                select {
                    value: "{training_data_size}",
                    onchange: move |e| training_data_size.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in training_sizes { option { value: "{x}", "{x}" } }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    value: "{notes}",
                    oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{task} | {model} | {language_family} | acc {accuracy}% | {training_data_size} tokens" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
