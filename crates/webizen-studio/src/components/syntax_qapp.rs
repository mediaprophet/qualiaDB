use dioxus::prelude::*;

#[component]
pub fn SyntaxQapp() -> Element {
    let mut syntactic_theory = use_signal(|| "Chomskyan Minimalism".to_string());
    let mut phrase_structure_rule = use_signal(|| String::new());
    let mut tree_notation = use_signal(|| String::new());
    let mut language_type = use_signal(|| "SVO".to_string());
    let mut movement_operation = use_signal(|| "Wh-Movement".to_string());
    let mut notes = use_signal(|| String::new());

    let theories = [
        "Chomskyan Minimalism",
        "Government & Binding",
        "HPSG",
        "LFG",
        "Construction Grammar",
        "Dependency Grammar",
        "Tree Adjoining Grammar",
        "Cognitive Grammar",
    ];
    let language_types = ["SVO", "SOV", "VSO", "VOS", "OVS", "OSV"];
    let movements = [
        "Wh-Movement",
        "NP Movement",
        "Head Movement",
        "Scrambling",
        "Topicalisation",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Syntax" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Syntactic Theory" }
                    select {
                        value: "{syntactic_theory}",
                        onchange: move |e| syntactic_theory.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in theories { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Language Type (Word Order)" }
                    select {
                        value: "{language_type}",
                        onchange: move |e| language_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in language_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Movement Operation" }
                    select {
                        value: "{movement_operation}",
                        onchange: move |e| movement_operation.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in movements { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Phrase Structure Rule" }
                textarea {
                    value: "{phrase_structure_rule}",
                    oninput: move |e| phrase_structure_rule.set(e.value()),
                    placeholder: "e.g. S → NP VP; VP → V NP PP; NP → Det N",
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px; font-family: monospace;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tree Notation (Bracket Notation)" }
                textarea {
                    value: "{tree_notation}",
                    oninput: move |e| tree_notation.set(e.value()),
                    placeholder: "[S [NP The cat] [VP [V chased] [NP the mouse]]]",
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px; font-family: monospace;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{syntactic_theory} | {language_type} | {movement_operation}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → syntactic analysis engine | phrase structure sieve | movement anchor" }
            }
        }
    }
}
