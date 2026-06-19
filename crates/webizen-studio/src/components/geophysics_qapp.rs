use dioxus::prelude::*;

#[component]
pub fn GeophysicsQapp() -> Element {
    let mut subfield = use_signal(|| "Seismology".to_string());
    let mut measurement_method = use_signal(|| "Seismic Reflection".to_string());
    let mut seismic_magnitude = use_signal(|| 6.5f64);
    let mut depth_km = use_signal(|| 10.0f64);
    let mut p_wave_velocity_kms = use_signal(|| 6.0f64);
    let mut s_wave_velocity_kms = use_signal(|| 3.5f64);
    let mut tectonic_plate = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    let subfields = [
        "Seismology",
        "Geomagnetism",
        "Gravity",
        "Geodesy",
        "Heat Flow",
        "Tectonophysics",
        "Exploration Geophysics",
    ];
    let methods = [
        "Seismic Reflection",
        "Refraction",
        "Gravity Survey",
        "Magnetic Survey",
        "Ground-Penetrating Radar",
        "MT",
        "InSAR",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Geophysics" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Subfield" }
                    select {
                        value: "{subfield}",
                        onchange: move |e| subfield.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in subfields { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Measurement Method" }
                    select {
                        value: "{measurement_method}",
                        onchange: move |e| measurement_method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in methods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Seismic Magnitude (Mw)" }
                    input {
                        r#type: "number",
                        step: "0.1",
                        value: "{seismic_magnitude}",
                        oninput: move |e| seismic_magnitude.set(e.value().parse().unwrap_or(6.5)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Depth (km)" }
                    input {
                        r#type: "number",
                        step: "0.5",
                        value: "{depth_km}",
                        oninput: move |e| depth_km.set(e.value().parse().unwrap_or(10.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "P-wave Velocity (km/s)" }
                    input {
                        r#type: "number",
                        step: "0.1",
                        value: "{p_wave_velocity_kms}",
                        oninput: move |e| p_wave_velocity_kms.set(e.value().parse().unwrap_or(6.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "S-wave Velocity (km/s)" }
                    input {
                        r#type: "number",
                        step: "0.1",
                        value: "{s_wave_velocity_kms}",
                        oninput: move |e| s_wave_velocity_kms.set(e.value().parse().unwrap_or(3.5)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tectonic Plate" }
                input {
                    r#type: "text",
                    value: "{tectonic_plate}",
                    oninput: move |e| tectonic_plate.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{subfield} | {measurement_method} | Mw={seismic_magnitude:.1} | depth={depth_km:.1}km | Vp={p_wave_velocity_kms:.1}km/s" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → seismic waveform engine | tectonic structure sieve | earth model graph" }
            }
        }
    }
}
