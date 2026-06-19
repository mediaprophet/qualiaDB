use dioxus::prelude::*;

#[component]
pub fn PublicHealthQapp() -> Element {
    let mut domain = use_signal(|| "Epidemiology".to_string());
    let mut intervention_level = use_signal(|| "Community".to_string());
    let mut study_design = use_signal(|| "Cohort".to_string());
    let mut r0 = use_signal(|| 2.5f64);
    let mut incidence_rate = use_signal(|| 0.05f64);
    let mut prevalence = use_signal(|| 0.1f64);
    let mut mortality_rate = use_signal(|| 0.02f64);
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Public Health QApp" }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Domain" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| domain.set(e.value()),
                    option { selected: true, "Epidemiology" }
                    option { "Biostatistics" }
                    option { "Environmental Health" }
                    option { "Health Policy" }
                    option { "Global Health" }
                    option { "Social Determinants" }
                    option { "Infectious Disease" }
                    option { "Chronic Disease" }
                    option { "Mental Health" }
                    option { "One Health" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Intervention Level" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| intervention_level.set(e.value()),
                    option { "Individual" }
                    option { selected: true, "Community" }
                    option { "Institutional" }
                    option { "Policy" }
                    option { "Environmental" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Study Design" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| study_design.set(e.value()),
                    option { "RCT" }
                    option { selected: true, "Cohort" }
                    option { "Case-Control" }
                    option { "Cross-Sectional" }
                    option { "Ecological" }
                    option { "Systematic Review" }
                    option { "Modelling" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "R\u{2080} (Reproductive Number): {r0():.2}" }
                input {
                    r#type: "range",
                    min: "0.0",
                    max: "20.0",
                    step: "0.1",
                    value: "{r0()}",
                    style: "width: 100%; box-sizing: border-box; accent-color: if r0() > 1.0 { \"var(--qualia-accent)\" } else { \"var(--qualia-accent)\" };",
                    oninput: move |e| r0.set(e.value().parse().unwrap_or(2.5)),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Incidence Rate: {incidence_rate():.3}" }
                input {
                    r#type: "range",
                    min: "0.0",
                    max: "1.0",
                    step: "0.001",
                    value: "{incidence_rate()}",
                    style: "width: 100%; box-sizing: border-box; accent-color: var(--qualia-accent);",
                    oninput: move |e| incidence_rate.set(e.value().parse().unwrap_or(0.05)),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Prevalence (0–1): {prevalence():.2}" }
                input {
                    r#type: "range",
                    min: "0.0",
                    max: "1.0",
                    step: "0.01",
                    value: "{prevalence()}",
                    style: "width: 100%; box-sizing: border-box; accent-color: var(--qualia-accent);",
                    oninput: move |e| prevalence.set(e.value().parse().unwrap_or(0.1)),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Mortality Rate: {mortality_rate():.3}" }
                input {
                    r#type: "range",
                    min: "0.0",
                    max: "1.0",
                    step: "0.001",
                    value: "{mortality_rate()}",
                    style: "width: 100%; box-sizing: border-box; accent-color: var(--qualia-accent);",
                    oninput: move |e| mortality_rate.set(e.value().parse().unwrap_or(0.02)),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box; min-height: 60px; resize: vertical;",
                    placeholder: "Additional notes...",
                    oninput: move |e| notes.set(e.value()),
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "QualiaDB Output" }
                div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); display: flex; flex-direction: column; gap: 4px;",
                    div { "Domain: {domain()}" }
                    div { "Design: {study_design()}" }
                    div { style: "color: if r0() > 1.0 { \"var(--qualia-accent)\" } else { \"var(--qualia-accent)\" };", "R\u{2080}: {r0():.2}" }
                    div { "Prevalence: {prevalence():.2}" }
                    div { "Mortality: {mortality_rate():.3}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → ODE epidemiological | statistical engine | graph network" }
            }
        }
    }
}
