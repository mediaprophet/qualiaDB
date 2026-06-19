use dioxus::prelude::*;

#[component]
pub fn DemographyAndPopulationStudiesQapp() -> Element {
    let mut demographic_transition = use_signal(|| "Post-Transition".to_string());
    let mut fertility_measure = use_signal(|| "Total Fertility Rate".to_string());
    let mut total_fertility_rate = use_signal(|| 2.1f64);
    let mut life_expectancy = use_signal(|| 72.0f64);
    let mut infant_mortality = use_signal(|| 15.0f64);
    let mut net_migration_rate = use_signal(|| 0.0f64);
    let mut dependency_ratio = use_signal(|| 50.0f64);
    let mut population_projection_method = use_signal(|| "Cohort-Component".to_string());
    let mut notes = use_signal(|| String::new());

    let transitions = [
        "Pre-Transition",
        "Transition",
        "Post-Transition",
        "Second Demographic Transition",
    ];
    let fertility_measures = [
        "Total Fertility Rate",
        "Net Reproduction Rate",
        "Cohort Fertility",
    ];
    let projection_methods = [
        "Cohort-Component",
        "Leslie Matrix",
        "Microsimulation",
        "Extrapolation",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Demography & Population Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Demographic Transition" }
                    select {
                        value: "{demographic_transition}",
                        onchange: move |e| demographic_transition.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in transitions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Fertility Measure" }
                    select {
                        value: "{fertility_measure}",
                        onchange: move |e| fertility_measure.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in fertility_measures { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Projection Method" }
                    select {
                        value: "{population_projection_method}",
                        onchange: move |e| population_projection_method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in projection_methods { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "TFR (0.8–6.0): {total_fertility_rate:.2}" }
                    input {
                        r#type: "range",
                        min: "0.8",
                        max: "6.0",
                        step: "0.1",
                        value: "{total_fertility_rate}",
                        oninput: move |e| total_fertility_rate.set(e.value().parse().unwrap_or(2.1)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Life Expectancy (years): {life_expectancy:.1}" }
                    input {
                        r#type: "number",
                        step: "0.5",
                        value: "{life_expectancy}",
                        oninput: move |e| life_expectancy.set(e.value().parse().unwrap_or(72.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Infant Mortality (per 1000)" }
                    input {
                        r#type: "number",
                        step: "0.1",
                        value: "{infant_mortality}",
                        oninput: move |e| infant_mortality.set(e.value().parse().unwrap_or(15.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Net Migration Rate (per 1000)" }
                    input {
                        r#type: "number",
                        step: "0.1",
                        value: "{net_migration_rate}",
                        oninput: move |e| net_migration_rate.set(e.value().parse().unwrap_or(0.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Dependency Ratio (%)" }
                    input {
                        r#type: "number",
                        step: "0.5",
                        value: "{dependency_ratio}",
                        oninput: move |e| dependency_ratio.set(e.value().parse().unwrap_or(50.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    value: "{notes}",
                    oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 50px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{demographic_transition} | TFR={total_fertility_rate:.2} | LE={life_expectancy:.1}yr | {population_projection_method}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → Leslie matrix engine | cohort projection sieve | demographic graph" }
            }
        }
    }
}
