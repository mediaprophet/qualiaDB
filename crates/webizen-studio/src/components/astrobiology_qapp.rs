use dioxus::prelude::*;

#[component]
pub fn AstrobiologyQapp() -> Element {
    let mut life_detection_method = use_signal(|| "Spectroscopic Biosignatures".to_string());
    let mut target_body = use_signal(|| "Mars".to_string());
    let mut habitable_zone_type = use_signal(|| "Circumstellar".to_string());
    let mut biomolecule = use_signal(|| "Amino Acid".to_string());
    let mut distance_ly = use_signal(|| 0.0f64);
    let mut temperature_k = use_signal(|| 273.0f64);
    let mut notes = use_signal(|| String::new());

    let methods = [
        "Spectroscopic Biosignatures",
        "Radio SETI",
        "Direct Sample",
        "Radar",
        "Gravitational Microlensing",
        "Transit Photometry",
    ];
    let targets = [
        "Mars",
        "Enceladus",
        "Europa",
        "Titan",
        "Exoplanet Habitable Zone",
        "Interstellar",
    ];
    let zone_types = [
        "Circumstellar",
        "Subsurface Ocean",
        "Atmospheric",
        "Hydrothermal",
    ];
    let biomolecules = [
        "DNA",
        "RNA",
        "Amino Acid",
        "Lipid",
        "ATP Analogue",
        "Unknown",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Astrobiology" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Life Detection Method" }
                    select {
                        value: "{life_detection_method}",
                        onchange: move |e| life_detection_method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in methods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Target Body" }
                    select {
                        value: "{target_body}",
                        onchange: move |e| target_body.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in targets { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Habitable Zone Type" }
                    select {
                        value: "{habitable_zone_type}",
                        onchange: move |e| habitable_zone_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in zone_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Biomolecule" }
                    select {
                        value: "{biomolecule}",
                        onchange: move |e| biomolecule.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in biomolecules { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Distance (light-years)" }
                    input {
                        r#type: "number",
                        value: "{distance_ly}",
                        oninput: move |e| distance_ly.set(e.value().parse().unwrap_or(0.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temperature (K)" }
                    input {
                        r#type: "number",
                        value: "{temperature_k}",
                        oninput: move |e| temperature_k.set(e.value().parse().unwrap_or(273.0)),
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
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{target_body} | {life_detection_method} | {biomolecule} | T: {temperature_k:.1}K" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → biosignature engine | habitability sieve | exoplanet graph" }
            }
        }
    }
}
