use dioxus::prelude::*;

#[component]
pub fn BioethicsQapp() -> Element {
    let mut domain = use_signal(|| "Clinical Ethics".to_string());
    let mut ethical_framework = use_signal(|| "Principilism (Beauchamp & Childress)".to_string());
    let mut four_principles = use_signal(|| "Autonomy".to_string());
    let mut epistemic_certainty = use_signal(|| 60u32);
    let mut dilemma_description = use_signal(|| String::new());
    let mut stakeholders = use_signal(|| String::new());

    let domains = [
        "Clinical Ethics",
        "Research Ethics",
        "Public Health Ethics",
        "Neuroethics",
        "Reproductive Ethics",
        "End-of-Life",
        "Enhancement Ethics",
        "AI in Medicine",
        "Global Bioethics",
    ];
    let frameworks = [
        "Principilism (Beauchamp & Childress)",
        "Casuistry",
        "Narrative Ethics",
        "Feminist Bioethics",
        "Utilitarian",
        "Kantian",
        "Virtue",
        "Buddhist",
        "Indigenous",
    ];
    let principles = [
        "Autonomy",
        "Beneficence",
        "Non-Maleficence",
        "Justice",
        "Autonomy + Beneficence",
        "All Four Principles",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Bioethics" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Domain" }
                    select {
                        value: "{domain}",
                        onchange: move |e| domain.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in domains { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Ethical Framework" }
                    select {
                        value: "{ethical_framework}",
                        onchange: move |e| ethical_framework.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in frameworks { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Four Principles Focus" }
                    select {
                        value: "{four_principles}",
                        onchange: move |e| four_principles.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in principles { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Epistemic Certainty (0–100): {epistemic_certainty}%" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "100",
                    value: "{epistemic_certainty}",
                    oninput: move |e| epistemic_certainty.set(e.value().parse().unwrap_or(60)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Dilemma Description" }
                textarea {
                    value: "{dilemma_description}",
                    oninput: move |e| dilemma_description.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 70px;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Stakeholders" }
                textarea {
                    value: "{stakeholders}",
                    oninput: move |e| stakeholders.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 50px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{domain} | {ethical_framework} | {four_principles} | Certainty: {epistemic_certainty}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → principilism engine | casuistry sieve | stakeholder graph" }
            }
        }
    }
}
