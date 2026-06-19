use dioxus::prelude::*;

#[component]
pub fn PovertyAndInequalityStudiesQapp() -> Element {
    let mut poverty_measure = use_signal(|| "Multidimensional Poverty Index".to_string());
    let mut inequality_metric = use_signal(|| "Gini".to_string());
    let mut theoretical_lens = use_signal(|| "Structural".to_string());
    let mut intervention = use_signal(|| "Universal Basic Income".to_string());
    let mut gini_coefficient = use_signal(|| 0.35f64);
    let mut poverty_headcount_pct = use_signal(|| 15.0f64);
    let mut notes = use_signal(|| String::new());

    let measures = [
        "Absolute Poverty Line",
        "Relative Poverty",
        "Multidimensional Poverty Index",
        "Capability Approach (Sen)",
        "Asset-Based",
    ];
    let metrics = [
        "Gini",
        "Palma Ratio",
        "Theil Index",
        "Atkinson Index",
        "Income Share Top 1%",
    ];
    let lenses = [
        "Structural",
        "Cultural",
        "Behavioural",
        "Feminist",
        "Postcolonial",
        "Rights-Based",
    ];
    let interventions = [
        "Cash Transfer",
        "Universal Basic Income",
        "Progressive Taxation",
        "Land Reform",
        "Education",
        "Healthcare",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Poverty & Inequality Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Poverty Measure" }
                    select {
                        value: "{poverty_measure}",
                        onchange: move |e| poverty_measure.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in measures { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Inequality Metric" }
                    select {
                        value: "{inequality_metric}",
                        onchange: move |e| inequality_metric.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in metrics { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Lens" }
                    select {
                        value: "{theoretical_lens}",
                        onchange: move |e| theoretical_lens.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in lenses { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Intervention" }
                    select {
                        value: "{intervention}",
                        onchange: move |e| intervention.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in interventions { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Gini Coefficient (0.0–1.0): {gini_coefficient:.2}" }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "1",
                        step: "0.01",
                        value: "{gini_coefficient}",
                        oninput: move |e| gini_coefficient.set(e.value().parse().unwrap_or(0.35)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Poverty Headcount %: {poverty_headcount_pct:.1}" }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "100",
                        step: "0.5",
                        value: "{poverty_headcount_pct}",
                        oninput: move |e| poverty_headcount_pct.set(e.value().parse().unwrap_or(15.0)),
                        style: "width: 100%; margin-top: 4px;"
                    }
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{poverty_measure} | {inequality_metric}: {gini_coefficient:.2} | Headcount: {poverty_headcount_pct:.1}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → inequality engine | capability approach sieve | distribution graph" }
            }
        }
    }
}
