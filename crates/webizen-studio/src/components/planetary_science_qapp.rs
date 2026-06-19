use dioxus::prelude::*;

#[component]
pub fn PlanetaryScienceQapp() -> Element {
    let mut body_type = use_signal(|| "Terrestrial Planet".to_string());
    let mut planet = use_signal(|| "Mars".to_string());
    let mut atmosphere_type = use_signal(|| "Thin CO2".to_string());
    let mut surface_feature = use_signal(|| "Impact Crater".to_string());
    let mut orbital_period_days = use_signal(|| 365.25f64);
    let mut distance_au = use_signal(|| 1.0f64);
    let mut notes = use_signal(|| String::new());

    let body_types = [
        "Terrestrial Planet",
        "Gas Giant",
        "Ice Giant",
        "Dwarf Planet",
        "Moon",
        "Asteroid",
        "Comet",
        "Trans-Neptunian Object",
    ];
    let planets = [
        "Mercury",
        "Venus",
        "Earth",
        "Mars",
        "Jupiter",
        "Saturn",
        "Uranus",
        "Neptune",
        "Pluto",
        "Exoplanet",
    ];
    let atmospheres = [
        "None",
        "Thin CO2",
        "Thick CO2",
        "N2-O2",
        "H2-He",
        "Methane-N2",
        "Thick SO2",
    ];
    let features = [
        "Impact Crater",
        "Volcano",
        "Canyon",
        "Ice Cap",
        "Ocean",
        "Desert",
        "Mountain Range",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Planetary Science" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Body Type" }
                    select {
                        value: "{body_type}",
                        onchange: move |e| body_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in body_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Planet / Target" }
                    select {
                        value: "{planet}",
                        onchange: move |e| planet.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in planets { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Atmosphere Type" }
                    select {
                        value: "{atmosphere_type}",
                        onchange: move |e| atmosphere_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in atmospheres { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Surface Feature" }
                    select {
                        value: "{surface_feature}",
                        onchange: move |e| surface_feature.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in features { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Orbital Period (days)" }
                    input {
                        r#type: "number",
                        step: "0.01",
                        value: "{orbital_period_days}",
                        oninput: move |e| orbital_period_days.set(e.value().parse().unwrap_or(365.25)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Distance (AU)" }
                    input {
                        r#type: "number",
                        step: "0.01",
                        value: "{distance_au}",
                        oninput: move |e| distance_au.set(e.value().parse().unwrap_or(1.0)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{planet} ({body_type}) | {atmosphere_type} | {orbital_period_days:.2}d | {distance_au:.2}AU" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → orbital mechanics engine | planetary geology sieve | solar system graph" }
            }
        }
    }
}
