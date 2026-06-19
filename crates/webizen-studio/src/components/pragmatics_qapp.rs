use dioxus::prelude::*;

#[component]
pub fn PragmaticsQapp() -> Element {
    let mut pragmatic_theory = use_signal(|| "Gricean Maxims".to_string());
    let mut speech_act_type = use_signal(|| "Assertive".to_string());
    let mut maxim_violated = use_signal(|| "Quantity".to_string());
    let mut face_threat_level = use_signal(|| 5u32);
    let mut utterance = use_signal(|| String::new());
    let mut context_notes = use_signal(|| String::new());

    let theories = [
        "Gricean Maxims",
        "Relevance Theory (Sperber & Wilson)",
        "Speech Act Theory (Austin/Searle)",
        "Politeness Theory (Brown & Levinson)",
        "Common Ground",
        "Implicature",
        "Presupposition",
        "Deixis",
        "Discourse Coherence",
    ];
    let speech_acts = [
        "Assertive",
        "Directive",
        "Commissive",
        "Expressive",
        "Declarative",
    ];
    let maxims = ["Quantity", "Quality", "Relation", "Manner"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Pragmatics" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Pragmatic Theory" }
                    select {
                        value: "{pragmatic_theory}",
                        onchange: move |e| pragmatic_theory.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in theories { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Speech Act Type" }
                    select {
                        value: "{speech_act_type}",
                        onchange: move |e| speech_act_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in speech_acts { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Maxim Violated" }
                    select {
                        value: "{maxim_violated}",
                        onchange: move |e| maxim_violated.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in maxims { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Face Threat Level (0-10): {face_threat_level}" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "10",
                    value: "{face_threat_level}",
                    oninput: move |e| face_threat_level.set(e.value().parse().unwrap_or(5)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Utterance Under Analysis" }
                textarea {
                    value: "{utterance}",
                    oninput: move |e| utterance.set(e.value()),
                    placeholder: "Enter the utterance to be pragmatically analysed...",
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Context Notes" }
                textarea {
                    value: "{context_notes}",
                    oninput: move |e| context_notes.set(e.value()),
                    placeholder: "Describe the situational context, participants, setting...",
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{pragmatic_theory} | {speech_act_type} | Maxim:{maxim_violated} | FTA:{face_threat_level}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → pragmatic inference engine | implicature sieve | speech act anchor" }
            }
        }
    }
}
