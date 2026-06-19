use dioxus::prelude::*;

#[component]
pub fn PhilosophyOfScienceQapp() -> Element {
    let mut topic = use_signal(|| "Scientific Realism".to_string());
    let mut position = use_signal(|| "Scientific Realism".to_string());
    let mut discipline_case_study = use_signal(|| String::new());
    let mut explanation_type = use_signal(|| "Causal".to_string());
    let mut notes = use_signal(|| String::new());

    let topics = [
        "Scientific Explanation",
        "Theory Choice",
        "Scientific Realism",
        "Underdetermination",
        "Induction",
        "Demarcation",
        "Reduction",
        "Emergence",
        "Values in Science",
        "Social Epistemology of Science",
    ];
    let positions = [
        "Scientific Realism",
        "Structural Realism",
        "Constructive Empiricism",
        "Instrumentalism",
        "Pragmatism",
        "Kuhnian Paradigm",
        "Lakatos MSRP",
        "Feyerabend Anarchism",
    ];
    let explanation_types = [
        "Deductive-Nomological",
        "Statistical-Relevance",
        "Causal",
        "Unification",
        "Pragmatic",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Philosophy of Science" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Topic" }
                    select {
                        value: "{topic}",
                        onchange: move |e| topic.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in topics { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Position" }
                    select {
                        value: "{position}",
                        onchange: move |e| position.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in positions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Explanation Type" }
                    select {
                        value: "{explanation_type}",
                        onchange: move |e| explanation_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in explanation_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Discipline Case Study" }
                    input {
                        r#type: "text",
                        value: "{discipline_case_study}",
                        oninput: move |e| discipline_case_study.set(e.value()),
                        placeholder: "e.g. Quantum Mechanics, Evolution, Climate Science",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{topic} | {position} | {explanation_type} | {discipline_case_study}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → scientific reasoning engine | demarcation sieve | explanation anchor" }
            }
        }
    }
}
