use dioxus::prelude::*;

#[component]
pub fn EnvironmentalChemistryQapp() -> Element {
    let mut compartment = use_signal(|| "Atmosphere".to_string());
    let mut pollutant_class = use_signal(|| "Heavy Metals".to_string());
    let mut process = use_signal(|| "Photolysis".to_string());
    let mut concentration = use_signal(|| 10.0f64);
    let mut toxicity = use_signal(|| 40u32);
    let mut remediation_efficiency = use_signal(|| 60u32);
    let mut notes = use_signal(|| String::new());

    let compartments = [
        "Atmosphere",
        "Hydrosphere",
        "Lithosphere",
        "Biosphere",
        "Urban Air",
        "Soil",
    ];
    let pollutant_classes = [
        "Heavy Metals",
        "POPs",
        "NOx/SOx",
        "Microplastics",
        "PFAS",
        "Radionuclides",
        "Pharmaceuticals",
    ];
    let processes = [
        "Photolysis",
        "Hydrolysis",
        "Biodegradation",
        "Bioaccumulation",
        "Sorption",
        "Volatilisation",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 {
                style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;",
                "Environmental Chemistry"
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Compartment" }
                select {
                    value: "{compartment}",
                    onchange: move |e| compartment.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in compartments { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Pollutant Class" }
                select {
                    value: "{pollutant_class}",
                    onchange: move |e| pollutant_class.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in pollutant_classes { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Process" }
                select {
                    value: "{process}",
                    onchange: move |e| process.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in processes { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Concentration (ppb): {concentration():.2}" }
                input {
                    r#type: "range", min: "0", max: "1000",
                    value: "{concentration()}",
                    oninput: move |e| concentration.set(e.value().parse::<f64>().unwrap_or(10.0)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Toxicity Index: {toxicity}" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{toxicity}",
                    oninput: move |e| toxicity.set(e.value().parse().unwrap_or(40)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Remediation Efficiency: {remediation_efficiency}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{remediation_efficiency}",
                    oninput: move |e| remediation_efficiency.set(e.value().parse().unwrap_or(60)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{compartment} | {pollutant_class} | {process} | {concentration():.2} ppb | tox {toxicity}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
