use dioxus::prelude::*;

#[component]
pub fn ChemicalPhysicsQapp() -> Element {
    let mut domain = use_signal(|| "Spectroscopy".to_string());
    let mut technique = use_signal(|| "NMR".to_string());
    let mut system = use_signal(|| "Gas Phase".to_string());
    let mut temperature_k = use_signal(|| 298.0f64);
    let mut pressure_atm = use_signal(|| 1.0f64);
    let mut notes = use_signal(|| String::new());

    let domains = [
        "Spectroscopy",
        "Reaction Dynamics",
        "Surface Science",
        "Statistical Mechanics",
        "Quantum Chemistry",
        "Photochemistry",
    ];
    let techniques = [
        "NMR",
        "IR",
        "Raman",
        "UV-Vis",
        "Mass Spectrometry",
        "X-ray Diffraction",
        "Time-resolved",
    ];
    let systems = [
        "Gas Phase",
        "Solution",
        "Solid Surface",
        "Cluster",
        "Nanoparticle",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 {
                style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;",
                "Chemical Physics"
            }

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
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Technique" }
                select {
                    value: "{technique}",
                    onchange: move |e| technique.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in techniques { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "System" }
                select {
                    value: "{system}",
                    onchange: move |e| system.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in systems { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temperature (K): {temperature_k():.1}" }
                input {
                    r#type: "range", min: "0", max: "5000",
                    value: "{temperature_k()}",
                    oninput: move |e| temperature_k.set(e.value().parse::<f64>().unwrap_or(298.0)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Pressure (atm): {pressure_atm():.2}" }
                input {
                    r#type: "range", min: "0", max: "1000",
                    value: "{pressure_atm()}",
                    oninput: move |e| pressure_atm.set(e.value().parse::<f64>().unwrap_or(1.0)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{domain} | {technique} | {system} | {temperature_k():.1} K | {pressure_atm():.2} atm" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
