use dioxus::prelude::*;

#[component]
pub fn EnvironmentalScienceQapp() -> Element {
    let mut domain = use_signal(|| "Climate Science".to_string());
    let mut pollutant_class = use_signal(|| "GHGs".to_string());
    let mut concentration = use_signal(|| 400.0f64);
    let mut co2_ppm = use_signal(|| 420.0f64);
    let mut temperature_anomaly_c = use_signal(|| 1.2f64);
    let mut mitigation_strategy = use_signal(|| "Carbon Capture".to_string());
    let mut monitoring_notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Environmental Science QApp" }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Domain" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| domain.set(e.value()),
                        option { "Climate Science" }
                        option { "Pollution" }
                        option { "Biodiversity Loss" }
                        option { "Land Use" }
                        option { "Water Resources" }
                        option { "Renewable Energy" }
                        option { "Environmental Policy" }
                        option { "Toxicology" }
                        option { "Deforestation" }
                        option { "Waste Management" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Pollutant Class" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| pollutant_class.set(e.value()),
                        option { "GHGs" }
                        option { "NOx" }
                        option { "PM2.5" }
                        option { "Heavy Metals" }
                        option { "Microplastics" }
                        option { "Persistent Organic" }
                        option { "Noise" }
                        option { "Light" }
                        option { "Eutrophicants" }
                        option { "VOCs" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "CO₂ (ppm): {co2_ppm:.0}" }
                    input {
                        r#type: "range",
                        min: "280",
                        max: "600",
                        step: "1",
                        value: "{co2_ppm}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| co2_ppm.set(e.value().parse().unwrap_or(420.0)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Pollutant Concentration" }
                    input {
                        r#type: "number",
                        value: "{concentration}",
                        step: "0.1",
                        min: "0",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| concentration.set(e.value().parse().unwrap_or(400.0)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temperature Anomaly (°C): {temperature_anomaly_c:.2}" }
                    input {
                        r#type: "range",
                        min: "-2.0",
                        max: "5.0",
                        step: "0.01",
                        value: "{temperature_anomaly_c}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| temperature_anomaly_c.set(e.value().parse().unwrap_or(1.2)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Mitigation Strategy" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| mitigation_strategy.set(e.value()),
                        option { "Carbon Capture" }
                        option { "Renewable Energy" }
                        option { "Reforestation" }
                        option { "Circular Economy" }
                        option { "Emissions Trading" }
                        option { "Green Infrastructure" }
                        option { "Bioremediation" }
                        option { "International Agreement" }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Monitoring Notes" }
                textarea {
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; min-height: 60px; box-sizing: border-box; resize: vertical;",
                    placeholder: "Enter sensor data, monitoring station details, policy context...",
                    oninput: move |e| monitoring_notes.set(e.value()),
                    "{monitoring_notes}"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Output / Analysis" }
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 0.8rem;",
                    div { style: "color: var(--qualia-text-muted);", "Domain:" }
                    div { style: "color: var(--qualia-text);", "{domain}" }
                    div { style: "color: var(--qualia-text-muted);", "Pollutant:" }
                    div { style: "color: var(--qualia-text);", "{pollutant_class}" }
                    div { style: "color: var(--qualia-text-muted);", "CO₂:" }
                    div { style: "color: var(--qualia-text);", "{co2_ppm:.0} ppm" }
                    div { style: "color: var(--qualia-text-muted);", "Temp Anomaly:" }
                    div { style: "color: var(--qualia-text);", "+{temperature_anomaly_c:.2}°C" }
                    div { style: "color: var(--qualia-text-muted);", "Mitigation:" }
                    div { style: "color: var(--qualia-text);", "{mitigation_strategy}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 12px; border-top: 1px solid var(--qualia-border); padding-top: 8px;",
                    "QualiaDB → ODE climate solver | Allen Interval temporal | geospatial sieve"
                }
            }
        }
    }
}
