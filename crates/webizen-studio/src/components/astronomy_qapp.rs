use dioxus::prelude::*;

#[component]
pub fn AstronomyQapp() -> Element {
    let mut celestial_object_type = use_signal(|| "Star".to_string());
    let mut observation_method = use_signal(|| "Optical".to_string());
    let mut telescope_aperture_m = use_signal(|| 1.0f64);
    let mut redshift_z = use_signal(|| 0.0f64);
    let mut magnitude = use_signal(|| 5.0f64);
    let mut spectral_class = use_signal(|| "G".to_string());
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Astronomy QApp" }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Celestial Object Type" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| celestial_object_type.set(e.value()),
                        option { "Star" }
                        option { "Planet" }
                        option { "Galaxy" }
                        option { "Nebula" }
                        option { "Pulsar" }
                        option { "Quasar" }
                        option { "Black Hole" }
                        option { "White Dwarf" }
                        option { "Neutron Star" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Observation Method" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| observation_method.set(e.value()),
                        option { "Optical" }
                        option { "Radio" }
                        option { "X-ray" }
                        option { "Infrared" }
                        option { "UV" }
                        option { "Gravitational Wave" }
                        option { "Gamma Ray" }
                        option { "Neutrino" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Spectral Class" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| spectral_class.set(e.value()),
                        option { "O" }
                        option { "B" }
                        option { "A" }
                        option { "F" }
                        option { "G" }
                        option { "K" }
                        option { "M" }
                        option { "L" }
                        option { "T" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Apparent Magnitude" }
                    input {
                        r#type: "number",
                        value: "{magnitude}",
                        step: "0.1",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| magnitude.set(e.value().parse().unwrap_or(5.0)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Redshift z" }
                    input {
                        r#type: "number",
                        value: "{redshift_z}",
                        step: "0.001",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| redshift_z.set(e.value().parse().unwrap_or(0.0)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Telescope Aperture (m): {telescope_aperture_m:.1}" }
                    input {
                        r#type: "range",
                        min: "0.1",
                        max: "39.3",
                        step: "0.1",
                        value: "{telescope_aperture_m}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| telescope_aperture_m.set(e.value().parse().unwrap_or(1.0)),
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Observation Notes" }
                textarea {
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; min-height: 60px; box-sizing: border-box; resize: vertical;",
                    placeholder: "Enter observation notes, coordinates, session details...",
                    oninput: move |e| notes.set(e.value()),
                    "{notes}"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Output / Analysis" }
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 0.8rem;",
                    div { style: "color: var(--qualia-text-muted);", "Object Type:" }
                    div { style: "color: var(--qualia-text);", "{celestial_object_type}" }
                    div { style: "color: var(--qualia-text-muted);", "Method:" }
                    div { style: "color: var(--qualia-text);", "{observation_method}" }
                    div { style: "color: var(--qualia-text-muted);", "Spectral Class:" }
                    div { style: "color: var(--qualia-text);", "{spectral_class}" }
                    div { style: "color: var(--qualia-text-muted);", "Magnitude:" }
                    div { style: "color: var(--qualia-text);", "{magnitude:.2}" }
                    div { style: "color: var(--qualia-text-muted);", "Redshift z:" }
                    div { style: "color: var(--qualia-text);", "{redshift_z:.4}" }
                    div { style: "color: var(--qualia-text-muted);", "Aperture:" }
                    div { style: "color: var(--qualia-text);", "{telescope_aperture_m:.1} m" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 12px; border-top: 1px solid var(--qualia-border); padding-top: 8px;",
                    "QualiaDB → Allen Interval Algebra | spectral analysis engine | numerical ODE solver"
                }
            }
        }
    }
}
