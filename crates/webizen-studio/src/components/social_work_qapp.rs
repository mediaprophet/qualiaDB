use dioxus::prelude::*;

#[component]
pub fn SocialWorkQapp() -> Element {
    let mut practice_level = use_signal(|| "Micro".to_string());
    let mut theoretical_framework = use_signal(|| "Strengths-Based".to_string());
    let mut client_population = use_signal(|| "Children & Families".to_string());
    let mut intervention_modality = use_signal(|| "Case Management".to_string());
    let mut risk_level = use_signal(|| "Medium".to_string());
    let mut burnout_risk = use_signal(|| 40u32);
    let mut case_notes = use_signal(|| String::new());

    let practice_levels = [
        "Micro",
        "Mezzo",
        "Macro",
        "Community Organising",
        "Policy Advocacy",
    ];
    let frameworks = [
        "Strengths-Based",
        "Trauma-Informed",
        "Systems Theory",
        "Ecological",
        "Feminist",
        "Anti-Oppressive",
        "Cognitive-Behavioural",
        "Psychodynamic",
        "Community Development",
    ];
    let populations = [
        "Children & Families",
        "Elderly",
        "People with Disabilities",
        "Refugees",
        "Homeless",
        "Addiction",
        "Mental Health",
        "Youth",
        "Veterans",
    ];
    let modalities = [
        "Case Management",
        "Group Work",
        "Community Development",
        "Advocacy",
        "Therapy",
        "Crisis Intervention",
    ];
    let risk_levels = ["Low", "Medium", "High", "Critical"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Social Work" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Practice Level" }
                    select {
                        value: "{practice_level}",
                        onchange: move |e| practice_level.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in practice_levels { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Framework" }
                    select {
                        value: "{theoretical_framework}",
                        onchange: move |e| theoretical_framework.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in frameworks { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Client Population" }
                    select {
                        value: "{client_population}",
                        onchange: move |e| client_population.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in populations { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Intervention Modality" }
                    select {
                        value: "{intervention_modality}",
                        onchange: move |e| intervention_modality.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in modalities { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Risk Level" }
                    select {
                        value: "{risk_level}",
                        onchange: move |e| risk_level.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in risk_levels { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Burnout Risk (0–100): {burnout_risk}%" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "100",
                    value: "{burnout_risk}",
                    oninput: move |e| burnout_risk.set(e.value().parse().unwrap_or(40)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{practice_level} | {theoretical_framework} | {client_population} | Risk: {risk_level} | Burnout: {burnout_risk}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → trauma-informed engine | risk assessment sieve | ecological systems graph" }
            }
        }
    }
}
