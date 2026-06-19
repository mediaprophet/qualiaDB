use dioxus::prelude::*;

#[component]
pub fn MetaphysicsQapp() -> Element {
    let mut metaphysical_domain = use_signal(|| "Ontology".to_string());
    let mut ontological_position = use_signal(|| "Materialism".to_string());
    let mut modal_framework = use_signal(|| "Possible Worlds (Lewis)".to_string());
    let mut temporal_view = use_signal(|| "Eternalism".to_string());
    let mut notes = use_signal(|| String::new());

    let domains = [
        "Ontology",
        "Philosophy of Time",
        "Modality",
        "Personal Identity",
        "Causation",
        "Free Will",
        "Mereology",
        "Abstract Objects",
        "Properties & Universals",
        "Persistence",
    ];
    let ontological_positions = [
        "Materialism",
        "Idealism",
        "Dualism",
        "Neutral Monism",
        "Panpsychism",
        "Process Philosophy",
        "Structural Realism",
        "Eliminativism",
    ];
    let modal_frameworks = [
        "Possible Worlds (Lewis)",
        "Ersatz Worlds",
        "Actualism",
        "Necessitarianism",
    ];
    let temporal_views = [
        "Presentism",
        "Eternalism",
        "Growing Block",
        "Moving Spotlight",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Metaphysics" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Metaphysical Domain" }
                    select {
                        value: "{metaphysical_domain}",
                        onchange: move |e| metaphysical_domain.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in domains { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Ontological Position" }
                    select {
                        value: "{ontological_position}",
                        onchange: move |e| ontological_position.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in ontological_positions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Modal Framework" }
                    select {
                        value: "{modal_framework}",
                        onchange: move |e| modal_framework.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in modal_frameworks { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temporal View" }
                    select {
                        value: "{temporal_view}",
                        onchange: move |e| temporal_view.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in temporal_views { option { value: "{x}", "{x}" } }
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{metaphysical_domain} | {ontological_position} | {modal_framework} | {temporal_view}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → ontology engine | modal logic sieve | metaphysical anchor" }
            }
        }
    }
}
