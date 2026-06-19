use dioxus::prelude::*;

#[component]
pub fn MedicalHumanitiesQapp() -> Element {
    let mut domain = use_signal(|| "Narrative Medicine".to_string());
    let mut condition_or_topic = use_signal(|| String::new());
    let mut historical_period = use_signal(|| "Contemporary".to_string());
    let mut methodological_approach = use_signal(|| "Close Reading".to_string());
    let mut clinical_context = use_signal(|| "Hospital".to_string());
    let mut notes = use_signal(|| String::new());

    let domains = [
        "Literature & Medicine",
        "Narrative Medicine",
        "History of Medicine",
        "Bioethics",
        "Patient Experience",
        "Social Determinants",
        "Disability & Medicine",
        "Mental Health Narratives",
        "Visual Medicine",
    ];
    let periods = [
        "Ancient",
        "Medieval",
        "Early Modern",
        "19th C.",
        "20th C.",
        "Contemporary",
    ];
    let approaches = [
        "Close Reading",
        "Ethnography",
        "Discourse Analysis",
        "Archive",
        "Oral History",
        "Policy",
    ];
    let contexts = [
        "Hospital",
        "Primary Care",
        "Palliative",
        "Public Health",
        "Global",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Medical Humanities" }

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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Historical Period" }
                    select {
                        value: "{historical_period}",
                        onchange: move |e| historical_period.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in periods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Methodological Approach" }
                    select {
                        value: "{methodological_approach}",
                        onchange: move |e| methodological_approach.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in approaches { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Clinical Context" }
                    select {
                        value: "{clinical_context}",
                        onchange: move |e| clinical_context.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in contexts { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Condition or Topic" }
                input {
                    r#type: "text",
                    value: "{condition_or_topic}",
                    oninput: move |e| condition_or_topic.set(e.value()),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{domain} | {historical_period} | {methodological_approach} | {clinical_context}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → narrative medicine engine | clinical ethics sieve | patient experience graph" }
            }
        }
    }
}
