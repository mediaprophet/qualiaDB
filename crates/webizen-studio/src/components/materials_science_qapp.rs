use dioxus::prelude::*;

#[component]
pub fn MaterialsScienceQapp() -> Element {
    let mut material_class = use_signal(|| "Metal".to_string());
    let mut property_focus = use_signal(|| "Mechanical".to_string());
    let mut characterisation_method = use_signal(|| "SEM".to_string());
    let mut temperature_c = use_signal(|| 25.0f64);
    let mut elastic_modulus_gpa = use_signal(|| 70.0f64);
    let mut hardness_vhn = use_signal(|| 100.0f64);
    let mut notes = use_signal(|| String::new());

    let material_classes = [
        "Metal",
        "Polymer",
        "Ceramic",
        "Composite",
        "Semiconductor",
        "Biomaterial",
        "Nanomaterial",
        "Metamaterial",
        "2D Material",
        "Amorphous",
    ];
    let property_focuses = [
        "Mechanical",
        "Thermal",
        "Electrical",
        "Magnetic",
        "Optical",
        "Chemical",
        "Biocompatibility",
    ];
    let char_methods = [
        "XRD",
        "SEM",
        "TEM",
        "AFM",
        "EDS",
        "Raman",
        "DSC",
        "XPS",
        "Nanoindentation",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Materials Science" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Material Class" }
                    select {
                        value: "{material_class}",
                        onchange: move |e| material_class.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in material_classes { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Property Focus" }
                    select {
                        value: "{property_focus}",
                        onchange: move |e| property_focus.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in property_focuses { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Characterisation Method" }
                    select {
                        value: "{characterisation_method}",
                        onchange: move |e| characterisation_method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in char_methods { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temperature (°C): {temperature_c:.1}" }
                input {
                    r#type: "range",
                    min: "-200",
                    max: "2000",
                    step: "5",
                    value: "{temperature_c()}",
                    oninput: move |e| temperature_c.set(e.value().parse().unwrap_or(25.0)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Elastic Modulus (GPa): {elastic_modulus_gpa:.1}" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "1000",
                    step: "1",
                    value: "{elastic_modulus_gpa()}",
                    oninput: move |e| elastic_modulus_gpa.set(e.value().parse().unwrap_or(70.0)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Hardness (VHN): {hardness_vhn:.0}" }
                input {
                    r#type: "range",
                    min: "1",
                    max: "10000",
                    step: "10",
                    value: "{hardness_vhn()}",
                    oninput: move |e| hardness_vhn.set(e.value().parse().unwrap_or(100.0)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{material_class} | {property_focus} | {characterisation_method} | E:{elastic_modulus_gpa:.0}GPa | H:{hardness_vhn:.0}VHN" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → materials characterisation engine | property sieve | microstructure anchor" }
            }
        }
    }
}
