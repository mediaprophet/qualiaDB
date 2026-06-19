use dioxus::prelude::*;

#[component]
pub fn ScreenPhilosophyQapp() -> Element {
    let mut philosophical_problem = use_signal(|| "Mind-Screen Analogy".to_string());
    let mut tradition = use_signal(|| "Continental".to_string());
    let mut screen_type = use_signal(|| "Cinema".to_string());
    let mut immersion = use_signal(|| 50u32);
    let mut reality_fidelity = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let problems = [
        "Mind-Screen Analogy",
        "Virtual Reality Ontology",
        "Cinematic Time",
        "Digital Image",
        "Interface as Phenomenology",
        "AI Representation",
    ];
    let traditions = [
        "Continental",
        "Analytic",
        "Cognitivism",
        "Phenomenology",
        "Pragmatism",
    ];
    let screen_types = [
        "Cinema",
        "Television",
        "Computer",
        "Mobile",
        "VR",
        "Public Screen",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Screen Philosophy" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Philosophical Problem" }
                select {
                    value: "{philosophical_problem}",
                    onchange: move |e| philosophical_problem.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in problems { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tradition" }
                select {
                    value: "{tradition}",
                    onchange: move |e| tradition.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in traditions { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Screen Type" }
                select {
                    value: "{screen_type}",
                    onchange: move |e| screen_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in screen_types { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Immersion: {immersion}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{immersion}",
                    oninput: move |e| immersion.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Reality Fidelity: {reality_fidelity}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{reality_fidelity}",
                    oninput: move |e| reality_fidelity.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{philosophical_problem} | {tradition} | {screen_type} | Immersion: {immersion} | Fidelity: {reality_fidelity}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → screen philosophy engine | image sieve | immersion anchor" }
            }
        }
    }
}
