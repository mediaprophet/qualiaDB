use dioxus::prelude::*;

#[component]
pub fn SocialPsychologyQapp() -> Element {
    let mut phenomenon = use_signal(|| "Conformity (Asch)".to_string());
    let mut study_design = use_signal(|| "Classic Experiment".to_string());
    let mut independent_variable = use_signal(|| String::new());
    let mut dependent_variable = use_signal(|| String::new());
    let mut sample_n = use_signal(|| 100u32);
    let mut effect_size_d = use_signal(|| 0.5f64);
    let mut p_value = use_signal(|| 0.05f64);
    let mut notes = use_signal(|| String::new());

    let phenomena = [
        "Conformity (Asch)",
        "Obedience (Milgram)",
        "Bystander Effect",
        "Cognitive Dissonance",
        "Attribution Error",
        "In-Group Bias",
        "Social Identity",
        "Stereotype Threat",
        "Priming",
        "Group Polarisation",
    ];
    let designs = [
        "Classic Experiment",
        "Replication",
        "Field Study",
        "Survey",
        "Meta-Analysis",
        "Qualitative",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Social Psychology" }

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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Study Design" }
                    select {
                        value: "{study_design}",
                        onchange: move |e| study_design.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in designs { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Independent Variable" }
                    input {
                        r#type: "text",
                        value: "{independent_variable}",
                        oninput: move |e| independent_variable.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Dependent Variable" }
                    input {
                        r#type: "text",
                        value: "{dependent_variable}",
                        oninput: move |e| dependent_variable.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sample N" }
                    input {
                        r#type: "number",
                        value: "{sample_n}",
                        oninput: move |e| sample_n.set(e.value().parse().unwrap_or(100)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Effect Size d" }
                    input {
                        r#type: "number",
                        step: "0.01",
                        value: "{effect_size_d}",
                        oninput: move |e| effect_size_d.set(e.value().parse().unwrap_or(0.5)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "p-value" }
                    input {
                        r#type: "number",
                        step: "0.001",
                        value: "{p_value}",
                        oninput: move |e| p_value.set(e.value().parse().unwrap_or(0.05)),
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
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{phenomenon} | {study_design} | N={sample_n} | d={effect_size_d:.2} | p={p_value:.3}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → social cognition engine | replication sieve | experimental design graph" }
            }
        }
    }
}
