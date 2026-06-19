use dioxus::prelude::*;

#[component]
pub fn ArcticStudiesQapp() -> Element {
    let mut region = use_signal(|| "Canadian Arctic".to_string());
    let mut indigenous_people = use_signal(|| String::new());
    let mut discipline = use_signal(|| "Environmental Science".to_string());
    let mut climate_focus = use_signal(|| "Sea Ice".to_string());
    let mut temperature_anomaly_c = use_signal(|| 3.0f64);
    let mut indigenous_knowledge_integration = use_signal(|| "Supplementary".to_string());
    let mut notes = use_signal(|| String::new());

    let regions = [
        "Alaska",
        "Canadian Arctic",
        "Greenland",
        "Svalbard",
        "Russian Arctic",
        "Fennoscandia Arctic",
    ];
    let disciplines = [
        "Environmental Science",
        "Anthropology",
        "History",
        "Political Science",
        "Indigenous Studies",
        "Oceanography",
    ];
    let climate_focuses = [
        "Sea Ice",
        "Permafrost",
        "Arctic Amplification",
        "Methane Release",
        "Glacial Retreat",
    ];
    let ik_options = ["None", "Supplementary", "Co-Equal", "Led-By-Community"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Arctic Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Region" }
                    select {
                        value: "{region}",
                        onchange: move |e| region.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in regions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Discipline" }
                    select {
                        value: "{discipline}",
                        onchange: move |e| discipline.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in disciplines { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Climate Focus" }
                    select {
                        value: "{climate_focus}",
                        onchange: move |e| climate_focus.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in climate_focuses { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Indigenous Knowledge Integration" }
                    select {
                        value: "{indigenous_knowledge_integration}",
                        onchange: move |e| indigenous_knowledge_integration.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in ik_options { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Indigenous People" }
                input {
                    r#type: "text",
                    value: "{indigenous_people}",
                    oninput: move |e| indigenous_people.set(e.value()),
                    placeholder: "e.g. Inuit, Sami, Nenets",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temperature Anomaly (°C): {temperature_anomaly_c:.1}" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "10",
                    step: "0.1",
                    value: "{temperature_anomaly_c}",
                    oninput: move |e| temperature_anomaly_c.set(e.value().parse().unwrap_or(3.0)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{region} | {discipline} | {climate_focus} | +{temperature_anomaly_c:.1}°C | IK: {indigenous_knowledge_integration}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → climate engine | indigenous knowledge sieve | geospatial anchor" }
            }
        }
    }
}
