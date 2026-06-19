use dioxus::prelude::*;

#[component]
pub fn PhilosophyOfMindQapp() -> Element {
    let mut mind_body_position = use_signal(|| "Functionalism".to_string());
    let mut problem = use_signal(|| "Hard Problem".to_string());
    let mut theory_of_consciousness = use_signal(|| "Global Workspace".to_string());
    let mut zombie_conceivable = use_signal(|| "Uncertain".to_string());
    let mut certainty = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let positions = [
        "Substance Dualism",
        "Property Dualism",
        "Physicalism",
        "Functionalism",
        "Eliminativism",
        "Anomalous Monism",
        "Neutral Monism",
        "Panpsychism",
        "Higher-Order Theory",
        "Representationalism",
    ];
    let problems = [
        "Hard Problem",
        "Explanatory Gap",
        "Qualia",
        "Intentionality",
        "Mental Causation",
        "Personal Identity",
        "Free Will",
        "Other Minds",
    ];
    let theories = [
        "Global Workspace",
        "IIT",
        "Higher-Order",
        "Predictive Processing",
        "Enactivism",
        "Quantum Consciousness",
        "None",
    ];
    let zombie_options = ["Conceivable", "Not Conceivable", "Uncertain"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Philosophy of Mind" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Mind-Body Position" }
                    select {
                        value: "{mind_body_position}",
                        onchange: move |e| mind_body_position.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in positions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Problem" }
                    select {
                        value: "{problem}",
                        onchange: move |e| problem.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in problems { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theory of Consciousness" }
                    select {
                        value: "{theory_of_consciousness}",
                        onchange: move |e| theory_of_consciousness.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in theories { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Zombie Conceivable" }
                    select {
                        value: "{zombie_conceivable}",
                        onchange: move |e| zombie_conceivable.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in zombie_options { option { value: "{x}", "{x}" } }
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
                    oninput: move |e| certainty.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{mind_body_position} | {problem} | {theory_of_consciousness} | Zombie:{zombie_conceivable} | {certainty}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → consciousness engine | qualia sieve | mind-body anchor" }
            }
        }
    }
}
