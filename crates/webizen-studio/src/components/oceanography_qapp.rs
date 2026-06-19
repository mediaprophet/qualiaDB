use dioxus::prelude::*;

#[component]
pub fn OceanographyQapp() -> Element {
    let mut subdiscipline = use_signal(|| "Physical".to_string());
    let mut ocean_basin = use_signal(|| "Pacific".to_string());
    let mut depth_m = use_signal(|| 200u32);
    let mut temperature_c = use_signal(|| 4.0f64);
    let mut salinity_ppt = use_signal(|| 35.0f64);
    let mut current_system = use_signal(|| "Gulf Stream".to_string());
    let mut dissolved_oxygen_mgl = use_signal(|| 6.5f64);
    let mut sampling_notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Oceanography QApp" }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Subdiscipline" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| subdiscipline.set(e.value()),
                        option { "Physical" }
                        option { "Chemical" }
                        option { "Biological" }
                        option { "Geological" }
                        option { "Paleoceanography" }
                        option { "Marine Ecology" }
                        option { "Climate Oceanography" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Ocean Basin" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| ocean_basin.set(e.value()),
                        option { "Pacific" }
                        option { "Atlantic" }
                        option { "Indian" }
                        option { "Arctic" }
                        option { "Southern" }
                        option { "Mediterranean" }
                        option { "Caribbean" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Current System" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| current_system.set(e.value()),
                        option { "Gulf Stream" }
                        option { "AMOC" }
                        option { "Kuroshio" }
                        option { "Antarctic Circumpolar" }
                        option { "El Niño" }
                        option { "La Niña" }
                        option { "Humboldt" }
                        option { "North Atlantic Gyre" }
                        option { "Indian Ocean Dipole" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Depth (m): {depth_m}" }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "11000",
                        step: "10",
                        value: "{depth_m}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| depth_m.set(e.value().parse().unwrap_or(200)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temperature (°C)" }
                    input {
                        r#type: "number",
                        value: "{temperature_c}",
                        step: "0.1",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| temperature_c.set(e.value().parse().unwrap_or(4.0)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Salinity (ppt)" }
                    input {
                        r#type: "number",
                        value: "{salinity_ppt}",
                        step: "0.1",
                        min: "0",
                        max: "50",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| salinity_ppt.set(e.value().parse().unwrap_or(35.0)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Dissolved O₂ (mg/L)" }
                    input {
                        r#type: "number",
                        value: "{dissolved_oxygen_mgl}",
                        step: "0.01",
                        min: "0",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| dissolved_oxygen_mgl.set(e.value().parse().unwrap_or(6.5)),
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sampling Notes" }
                textarea {
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; min-height: 60px; box-sizing: border-box; resize: vertical;",
                    placeholder: "CTD cast data, cruise details, instrument calibration, sample preservation...",
                    oninput: move |e| sampling_notes.set(e.value()),
                    "{sampling_notes}"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Output / Analysis" }
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 0.8rem;",
                    div { style: "color: var(--qualia-text-muted);", "Subdiscipline:" }
                    div { style: "color: var(--qualia-text);", "{subdiscipline}" }
                    div { style: "color: var(--qualia-text-muted);", "Basin:" }
                    div { style: "color: var(--qualia-text);", "{ocean_basin}" }
                    div { style: "color: var(--qualia-text-muted);", "Current System:" }
                    div { style: "color: var(--qualia-text);", "{current_system}" }
                    div { style: "color: var(--qualia-text-muted);", "Depth:" }
                    div { style: "color: var(--qualia-text);", "{depth_m} m" }
                    div { style: "color: var(--qualia-text-muted);", "T / S:" }
                    div { style: "color: var(--qualia-text);", "{temperature_c:.1}°C / {salinity_ppt:.1} ppt" }
                    div { style: "color: var(--qualia-text-muted);", "DO:" }
                    div { style: "color: var(--qualia-text);", "{dissolved_oxygen_mgl:.2} mg/L" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 12px; border-top: 1px solid var(--qualia-border); padding-top: 8px;",
                    "QualiaDB → ODE fluid dynamics solver | Allen Interval temporal | geochemical sieve"
                }
            }
        }
    }
}
