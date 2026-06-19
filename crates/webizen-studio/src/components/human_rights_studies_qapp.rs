use dioxus::prelude::*;

#[component]
pub fn HumanRightsStudiesQapp() -> Element {
    let mut rights_category = use_signal(|| "Civil".to_string());
    let mut violation_type = use_signal(|| "Arbitrary Detention".to_string());
    let mut enforcement_mechanism = use_signal(|| "UN Treaty Body".to_string());
    let mut political_regime = use_signal(|| "Hybrid".to_string());
    let mut severity = use_signal(|| 3u32);
    let mut case_notes = use_signal(|| String::new());

    let rights_categories = [
        "Civil",
        "Political",
        "Economic",
        "Social",
        "Cultural",
        "Environmental",
        "Collective",
        "Emerging Digital",
    ];
    let violations = [
        "Torture",
        "Extrajudicial Killing",
        "Arbitrary Detention",
        "Discrimination",
        "Statelessness",
        "Forced Displacement",
        "Child Labour",
        "Trafficking",
    ];
    let mechanisms = [
        "UN Treaty Body",
        "ICC",
        "Regional Court",
        "National NHR Institution",
        "NGO Advocacy",
        "Universal Periodic Review",
    ];
    let regimes = ["Democracy", "Hybrid", "Autocracy"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Human Rights Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Rights Category" }
                    select {
                        value: "{rights_category}",
                        onchange: move |e| rights_category.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in rights_categories { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Violation Type" }
                    select {
                        value: "{violation_type}",
                        onchange: move |e| violation_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in violations { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Enforcement Mechanism" }
                    select {
                        value: "{enforcement_mechanism}",
                        onchange: move |e| enforcement_mechanism.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in mechanisms { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Political Regime" }
                    select {
                        value: "{political_regime}",
                        onchange: move |e| political_regime.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in regimes { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Severity (1–5): {severity}" }
                input {
                    r#type: "range",
                    min: "1",
                    max: "5",
                    value: "{severity}",
                    oninput: move |e| severity.set(e.value().parse().unwrap_or(3)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Case Notes" }
                textarea {
                    value: "{case_notes}",
                    oninput: move |e| case_notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 80px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{rights_category} | {violation_type} | {enforcement_mechanism} | Severity: {severity}/5" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → accountability graph | treaty body sieve | regime classification engine" }
            }
        }
    }
}
