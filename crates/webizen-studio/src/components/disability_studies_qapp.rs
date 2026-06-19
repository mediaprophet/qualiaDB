use dioxus::prelude::*;

#[component]
pub fn DisabilityStudiesQapp() -> Element {
    let mut model = use_signal(|| "Social".to_string());
    let mut disability_category = use_signal(|| "Physical".to_string());
    let mut accessibility_domain = use_signal(|| "Physical".to_string());
    let mut methodology = use_signal(|| "Policy Analysis".to_string());
    let mut policy_context = use_signal(|| "ADA".to_string());
    let mut notes = use_signal(|| String::new());

    let models = [
        "Medical",
        "Social",
        "Cultural",
        "Critical",
        "Crip Theory",
        "Neurodiversity",
        "Intersectional",
        "Poststructuralist",
    ];
    let categories = [
        "Physical",
        "Sensory",
        "Cognitive",
        "Psychiatric",
        "Chronic Illness",
        "Neurodivergent",
        "Multiple",
    ];
    let domains = [
        "Physical",
        "Digital",
        "Social",
        "Institutional",
        "Legal",
        "Communicative",
    ];
    let methodologies = [
        "Autoethnography",
        "Survey",
        "Policy Analysis",
        "Critical Discourse",
        "Historical",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Disability Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Model" }
                    select {
                        value: "{model}",
                        onchange: move |e| model.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in models { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Disability Category" }
                    select {
                        value: "{disability_category}",
                        onchange: move |e| disability_category.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in categories { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Accessibility Domain" }
                    select {
                        value: "{accessibility_domain}",
                        onchange: move |e| accessibility_domain.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in domains { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Methodology" }
                    select {
                        value: "{methodology}",
                        onchange: move |e| methodology.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in methodologies { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Policy Context (e.g. ADA, CRPD, NDIS)" }
                input {
                    r#type: "text",
                    value: "{policy_context}",
                    oninput: move |e| policy_context.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{model} Model | {disability_category} | {accessibility_domain} | {policy_context}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → accessibility sieve | intersectional engine | policy graph" }
            }
        }
    }
}
