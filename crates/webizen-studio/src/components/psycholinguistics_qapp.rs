use dioxus::prelude::*;

#[component]
pub fn PsycholinguisticsQapp() -> Element {
    let mut phenomenon = use_signal(|| "Sentence Processing".to_string());
    let mut theoretical_model = use_signal(|| "Constraint-Based".to_string());
    let mut method = use_signal(|| "Eye Tracking".to_string());
    let mut reaction_time_ms = use_signal(|| 500u32);
    let mut notes = use_signal(|| String::new());

    let phenomena = [
        "Language Acquisition",
        "Word Recognition",
        "Sentence Processing",
        "Discourse Comprehension",
        "Speech Production",
        "Bilingualism",
        "Aphasia",
        "Reading",
        "Metaphor Processing",
        "Pragmatic Inference",
    ];
    let models = [
        "Parallel Distributed Processing",
        "Serial Stage Model",
        "Interactive Activation",
        "Constraint-Based",
        "Usage-Based",
        "Generativist",
        "Embodied Language",
    ];
    let methods = [
        "Eye Tracking",
        "ERP",
        "fMRI",
        "Priming",
        "Self-Paced Reading",
        "Diary Study",
        "Corpus",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Psycholinguistics" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Phenomenon" }
                    select {
                        value: "{phenomenon}",
                        onchange: move |e| phenomenon.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in phenomena { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Model" }
                    select {
                        value: "{theoretical_model}",
                        onchange: move |e| theoretical_model.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in models { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Method" }
                    select {
                        value: "{method}",
                        onchange: move |e| method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in methods { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Reaction Time (ms): {reaction_time_ms}" }
                input {
                    r#type: "range",
                    min: "100",
                    max: "2000",
                    value: "{reaction_time_ms}",
                    oninput: move |e| reaction_time_ms.set(e.value().parse().unwrap_or(500)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    value: "{notes}",
                    oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 80px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{phenomenon} | {theoretical_model} | {method} | RT:{reaction_time_ms}ms" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → psycholinguistics engine | processing sieve | neurocognitive anchor" }
            }
        }
    }
}
