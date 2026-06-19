use dioxus::prelude::*;

#[component]
pub fn GerontologyQapp() -> Element {
    let mut subfield = use_signal(|| "Social".to_string());
    let mut life_stage = use_signal(|| "Old 75-84".to_string());
    let mut theory_of_ageing = use_signal(|| "Activity Theory".to_string());
    let mut health_domain = use_signal(|| "Cognitive Health".to_string());
    let mut age_in_study = use_signal(|| 75u32);
    let mut methodology = use_signal(|| "Longitudinal Cohort".to_string());
    let mut notes = use_signal(|| String::new());

    let subfields = [
        "Social",
        "Biological",
        "Clinical",
        "Environmental",
        "Critical",
        "Global",
        "Feminist",
    ];
    let life_stages = ["Young-Old 65-74", "Old 75-84", "Oldest-Old 85+"];
    let theories = [
        "Disengagement",
        "Activity Theory",
        "Continuity",
        "Successful Ageing",
        "Critical Gerontology",
        "Compression of Morbidity",
        "Cumulative Disadvantage",
    ];
    let health_domains = [
        "Cognitive Health",
        "Physical Mobility",
        "Chronic Disease",
        "Mental Health",
        "Social Isolation",
        "End-of-Life",
    ];
    let methods = [
        "Longitudinal Cohort",
        "Cross-Sectional",
        "Qualitative",
        "Clinical Trial",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Gerontology" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Subfield" }
                    select {
                        value: "{subfield}",
                        onchange: move |e| subfield.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in subfields { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Life Stage" }
                    select {
                        value: "{life_stage}",
                        onchange: move |e| life_stage.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in life_stages { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theory of Ageing" }
                    select {
                        value: "{theory_of_ageing}",
                        onchange: move |e| theory_of_ageing.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in theories { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Health Domain" }
                    select {
                        value: "{health_domain}",
                        onchange: move |e| health_domain.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in health_domains { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Methodology" }
                    select {
                        value: "{methodology}",
                        onchange: move |e| methodology.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in methods { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Age in Study: {age_in_study}" }
                input {
                    r#type: "range",
                    min: "60",
                    max: "110",
                    value: "{age_in_study}",
                    oninput: move |e| age_in_study.set(e.value().parse().unwrap_or(75)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{subfield} | {life_stage} | {theory_of_ageing} | Age:{age_in_study} | {health_domain}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → gerontology engine | ageing theory sieve | health anchor" }
            }
        }
    }
}
