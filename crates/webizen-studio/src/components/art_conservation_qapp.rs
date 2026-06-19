use dioxus::prelude::*;

#[component]
pub fn ArtConservationQapp() -> Element {
    let mut object_type = use_signal(|| "Panel Painting".to_string());
    let mut condition_assessment = use_signal(|| "Good".to_string());
    let mut treatment_approach = use_signal(|| "Preventive Conservation".to_string());
    let mut dating_method = use_signal(|| "XRF".to_string());
    let mut environmental_rh_pct = use_signal(|| 50.0f64);
    let mut temperature_c = use_signal(|| 18.0f64);
    let mut provenance_confidence = use_signal(|| 75u32);
    let mut notes = use_signal(|| String::new());

    let object_types = [
        "Panel Painting",
        "Canvas Painting",
        "Works on Paper",
        "Textile",
        "Ceramics",
        "Glass",
        "Metal",
        "Stone",
        "Organic Material",
        "Photographic",
        "Born-Digital",
    ];
    let conditions = ["Excellent", "Good", "Fair", "Poor", "Critical"];
    let treatments = [
        "Preventive Conservation",
        "Consolidation",
        "Cleaning",
        "Inpainting",
        "Structural Repair",
        "Environmental Control",
        "Digital Preservation",
    ];
    let dating_methods = [
        "XRF",
        "UV",
        "IR Reflectography",
        "Carbon-14",
        "Dendrochronology",
        "Stylistic",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Art Conservation" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Object Type" }
                    select {
                        value: "{object_type}",
                        onchange: move |e| object_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in object_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Condition Assessment" }
                    select {
                        value: "{condition_assessment}",
                        onchange: move |e| condition_assessment.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in conditions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Treatment Approach" }
                    select {
                        value: "{treatment_approach}",
                        onchange: move |e| treatment_approach.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in treatments { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Dating Method" }
                    select {
                        value: "{dating_method}",
                        onchange: move |e| dating_method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in dating_methods { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "RH % (30–70): {environmental_rh_pct:.0}" }
                    input {
                        r#type: "range",
                        min: "30",
                        max: "70",
                        step: "1",
                        value: "{environmental_rh_pct}",
                        oninput: move |e| environmental_rh_pct.set(e.value().parse().unwrap_or(50.0)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temperature °C: {temperature_c:.1}" }
                    input {
                        r#type: "number",
                        step: "0.5",
                        value: "{temperature_c}",
                        oninput: move |e| temperature_c.set(e.value().parse().unwrap_or(18.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Provenance Confidence (0–100): {provenance_confidence}%" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "100",
                    value: "{provenance_confidence}",
                    oninput: move |e| provenance_confidence.set(e.value().parse().unwrap_or(75)),
                    style: "width: 100%; margin-top: 4px;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{object_type} | {condition_assessment} | {treatment_approach} | RH={environmental_rh_pct:.0}% | Prov={provenance_confidence}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → material analysis engine | provenance graph | environmental monitoring sieve" }
            }
        }
    }
}
