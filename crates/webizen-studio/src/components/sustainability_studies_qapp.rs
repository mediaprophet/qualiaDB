use dioxus::prelude::*;

#[component]
pub fn SustainabilityStudiesQapp() -> Element {
    let mut domain = use_signal(|| "Climate Mitigation".to_string());
    let mut framework = use_signal(|| "Planetary Boundaries".to_string());
    let mut indicator_focus = use_signal(|| "Carbon Footprint".to_string());
    let mut co2_ppm = use_signal(|| 420.0f64);
    let mut temperature_target_c = use_signal(|| "1.5".to_string());
    let mut notes = use_signal(|| String::new());

    let domains = [
        "Climate Mitigation",
        "Climate Adaptation",
        "Circular Economy",
        "Sustainable Agriculture",
        "Urban Sustainability",
        "Ocean Health",
        "Biodiversity",
        "Energy Transition",
        "Water Security",
        "Sustainable Finance",
    ];
    let frameworks = [
        "SDGs",
        "Planetary Boundaries",
        "Doughnut Economics",
        "Natural Capital",
        "One Health",
        "Just Transition",
        "Degrowth",
    ];
    let indicators = [
        "Carbon Footprint",
        "Ecological Footprint",
        "Water Footprint",
        "HDI",
        "Genuine Progress Indicator",
    ];
    let temp_targets = ["1.5", "2.0", "3.0"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Sustainability Studies" }

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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Framework" }
                    select {
                        value: "{framework}",
                        onchange: move |e| framework.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in frameworks { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Indicator Focus" }
                    select {
                        value: "{indicator_focus}",
                        onchange: move |e| indicator_focus.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in indicators { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temperature Target °C" }
                    select {
                        value: "{temperature_target_c}",
                        onchange: move |e| temperature_target_c.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in temp_targets { option { value: "{x}", "{x}°C" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "CO₂ Concentration (ppm): {co2_ppm:.0}" }
                input {
                    r#type: "range",
                    min: "280",
                    max: "600",
                    step: "1",
                    value: "{co2_ppm}",
                    oninput: move |e| co2_ppm.set(e.value().parse().unwrap_or(420.0)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{domain} | {framework} | {indicator_focus} | CO₂: {co2_ppm:.0}ppm | Target: {temperature_target_c}°C" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → planetary boundaries engine | carbon accounting sieve | SDG graph" }
            }
        }
    }
}
