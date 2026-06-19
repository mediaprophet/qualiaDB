use dioxus::prelude::*;

#[component]
pub fn GeochemistryQapp() -> Element {
    let mut sub_discipline = use_signal(|| "Isotope Geochemistry".to_string());
    let mut element_system = use_signal(|| "Carbon".to_string());
    let mut reservoir = use_signal(|| "Mantle".to_string());
    let mut temperature_c = use_signal(|| 300.0f64);
    let mut pressure_gpa = use_signal(|| 1.0f64);
    let mut notes = use_signal(|| String::new());

    let sub_disciplines = [
        "Isotope Geochemistry",
        "Organic Geochemistry",
        "Marine Geochemistry",
        "Planetary Geochemistry",
        "Hydrogeochemistry",
        "Cosmochemistry",
    ];
    let element_systems = [
        "Carbon",
        "Oxygen",
        "Sulfur",
        "Strontium",
        "Lead",
        "REE",
        "Platinum Group",
    ];
    let reservoirs = ["Mantle", "Crust", "Ocean", "Atmosphere", "Sediment", "Ice"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 {
                style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;",
                "Geochemistry"
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sub-discipline" }
                select {
                    value: "{sub_discipline}",
                    onchange: move |e| sub_discipline.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in sub_disciplines { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Element / System" }
                select {
                    value: "{element_system}",
                    onchange: move |e| element_system.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in element_systems { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Reservoir" }
                select {
                    value: "{reservoir}",
                    onchange: move |e| reservoir.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in reservoirs { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temperature (°C): {temperature_c():.1}" }
                input {
                    r#type: "range", min: "0", max: "1500",
                    value: "{temperature_c()}",
                    oninput: move |e| temperature_c.set(e.value().parse::<f64>().unwrap_or(300.0)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Pressure (GPa): {pressure_gpa():.2}" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{pressure_gpa()}",
                    oninput: move |e| pressure_gpa.set(e.value().parse::<f64>().unwrap_or(1.0)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{sub_discipline} | {element_system} | {reservoir} | {temperature_c():.1}°C | {pressure_gpa():.2} GPa" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
