use dioxus::prelude::*;

#[component]
pub fn PenologyQapp() -> Element {
    let mut sentencing_philosophy = use_signal(|| "Rehabilitation".to_string());
    let mut incarceration_type = use_signal(|| "Medium Security".to_string());
    let mut reform_approach = use_signal(|| "Education Programmes".to_string());
    let mut recidivism_rate = use_signal(|| 0.4f64);
    let mut sentence_length_months = use_signal(|| 24u32);
    let mut overcrowding_pct = use_signal(|| 110u32);
    let mut notes = use_signal(|| String::new());

    let philosophies = [
        "Deterrence",
        "Retribution",
        "Rehabilitation",
        "Incapacitation",
        "Restoration",
    ];
    let incarceration_types = [
        "Maximum Security",
        "Medium Security",
        "Minimum Security",
        "Open",
        "Remand",
        "Juvenile",
        "Immigration Detention",
    ];
    let reform_approaches = [
        "Education Programmes",
        "Vocational Training",
        "Therapeutic Communities",
        "Restorative Justice",
        "Abolition",
        "Decarceration",
        "Electronic Monitoring",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Penology" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sentencing Philosophy" }
                    select {
                        value: "{sentencing_philosophy}",
                        onchange: move |e| sentencing_philosophy.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in philosophies { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Incarceration Type" }
                    select {
                        value: "{incarceration_type}",
                        onchange: move |e| incarceration_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in incarceration_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Reform Approach" }
                    select {
                        value: "{reform_approach}",
                        onchange: move |e| reform_approach.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in reform_approaches { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Recidivism Rate: {recidivism_rate:.2}" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "100",
                    value: "{recidivism_rate() * 100.0}",
                    oninput: move |e| recidivism_rate.set(e.value().parse::<f64>().unwrap_or(40.0) / 100.0),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sentence Length (months): {sentence_length_months}" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "600",
                    value: "{sentence_length_months}",
                    oninput: move |e| sentence_length_months.set(e.value().parse().unwrap_or(24)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Overcrowding %: {overcrowding_pct}" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "200",
                    value: "{overcrowding_pct}",
                    oninput: move |e| overcrowding_pct.set(e.value().parse().unwrap_or(110)),
                    style: "width: 100%; margin-top: 4px;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{sentencing_philosophy} | {incarceration_type} | Recidivism:{recidivism_rate:.2} | {sentence_length_months}mo | {overcrowding_pct}% capacity" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → penological engine | recidivism sieve | reform anchor" }
            }
        }
    }
}
