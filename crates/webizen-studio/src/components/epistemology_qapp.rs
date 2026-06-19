use dioxus::prelude::*;

#[component]
pub fn EpistemologyQapp() -> Element {
    let mut epistemic_theory = use_signal(|| "Reliabilism".to_string());
    let mut knowledge_type = use_signal(|| "A Posteriori".to_string());
    let mut justification_type = use_signal(|| "Externalist".to_string());
    let mut sceptical_challenge = use_signal(|| "Gettier Problem".to_string());
    let mut certainty = use_signal(|| 60u32);
    let mut proposition = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    let theories = [
        "Foundationalism",
        "Coherentism",
        "Reliabilism",
        "Virtue Epistemology",
        "Pragmatism",
        "Contextualism",
        "Social Epistemology",
        "Reformed Epistemology",
        "Feminist",
        "Evolutionary",
    ];
    let knowledge_types = [
        "A Priori",
        "A Posteriori",
        "Propositional",
        "Procedural",
        "Tacit",
        "Testimonial",
    ];
    let justification_types = ["Internalist", "Externalist", "Evidentialist", "Dogmatist"];
    let challenges = [
        "Brain-in-Vat",
        "Dream Argument",
        "Gettier Problem",
        "Underdetermination",
        "Closure Failure",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Epistemology" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Epistemic Theory" }
                    select {
                        value: "{epistemic_theory}",
                        onchange: move |e| epistemic_theory.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in theories { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Knowledge Type" }
                    select {
                        value: "{knowledge_type}",
                        onchange: move |e| knowledge_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in knowledge_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Justification Type" }
                    select {
                        value: "{justification_type}",
                        onchange: move |e| justification_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in justification_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sceptical Challenge" }
                    select {
                        value: "{sceptical_challenge}",
                        onchange: move |e| sceptical_challenge.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in challenges { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Certainty: {certainty}%" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "100",
                    value: "{certainty}",
                    oninput: move |e| certainty.set(e.value().parse().unwrap_or(60)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Proposition" }
                textarea {
                    value: "{proposition}",
                    oninput: move |e| proposition.set(e.value()),
                    placeholder: "Enter the epistemic proposition under analysis...",
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{epistemic_theory} | {knowledge_type} | {justification_type} | {sceptical_challenge} | Certainty:{certainty}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → epistemic engine | justification sieve | knowledge anchor" }
            }
        }
    }
}
