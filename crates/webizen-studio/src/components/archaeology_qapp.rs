use dioxus::prelude::*;

#[component]
pub fn ArchaeologyQapp() -> Element {
    let mut period = use_signal(|| "Iron Age".to_string());
    let mut region = use_signal(|| String::new());
    let mut site_type = use_signal(|| "Settlement".to_string());
    let mut dating_method = use_signal(|| "Radiocarbon (¹⁴C)".to_string());
    let mut date_bp = use_signal(|| 2500u32);
    let mut date_uncertainty = use_signal(|| 50u32);
    let mut stratigraphy_layer = use_signal(|| String::new());
    let mut find_category = use_signal(|| "Ceramics".to_string());
    let mut excavation_notes = use_signal(|| String::new());

    let periods = [
        "Lower Palaeolithic (3.3Ma–300Ka)",
        "Middle Palaeolithic (300–45Ka)",
        "Upper Palaeolithic (45–10Ka)",
        "Mesolithic (10–5Ka BCE)",
        "Neolithic (5–3Ka BCE)",
        "Chalcolithic / Copper Age",
        "Bronze Age",
        "Iron Age",
        "Classical Antiquity",
        "Late Antiquity / Migration Period",
        "Medieval",
        "Early Modern",
        "Colonial / Historical",
    ];
    let site_types = [
        "Settlement",
        "Tell / Mound",
        "Cave / Rock Shelter",
        "Burial Ground / Necropolis",
        "Ritual / Ceremonial Site",
        "Fortification",
        "Production Site / Workshop",
        "Agricultural Field System",
        "Shipwreck / Underwater",
        "Palimpsest / Multi-Period",
    ];
    let dating_methods = [
        "Radiocarbon (¹⁴C)",
        "Optically Stimulated Luminescence (OSL)",
        "Thermoluminescence (TL)",
        "Dendrochronology",
        "Archaeomagnetism",
        "Potassium-Argon (K-Ar)",
        "Uranium-Series",
        "Typological / Seriation",
        "Stratigraphy (Relative)",
        "Historical Record",
    ];
    let finds = [
        "Ceramics",
        "Lithics / Flint",
        "Bone / Faunal Remains",
        "Archaeobotanical Remains",
        "Metal Artefacts",
        "Glass",
        "Textile / Organic",
        "Coins / Numismatics",
        "Inscriptions / Epigraphy",
        "Human Skeletal Remains",
        "Architectural Remains",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Archaeology" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Archaeological Period" }
                    select {
                        value: "{period}",
                        onchange: move |e| period.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in periods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Site Type" }
                    select {
                        value: "{site_type}",
                        onchange: move |e| site_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in site_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Find Category" }
                    select {
                        value: "{find_category}",
                        onchange: move |e| find_category.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in finds { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Dating Method" }
                    select {
                        value: "{dating_method}",
                        onchange: move |e| dating_method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in dating_methods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Region / Site Name" }
                    input {
                        r#type: "text", placeholder: "e.g. Çatalhöyük, Levant coastal plain…",
                        value: "{region}",
                        oninput: move |e| region.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Date (BP): {date_bp}" }
                    input {
                        r#type: "range", min: "0", max: "3300000",
                        value: "{date_bp}",
                        oninput: move |e| date_bp.set(e.value().parse().unwrap_or(2500)),
                        style: "width: 100%; margin-top: 10px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "±Uncertainty (yr): {date_uncertainty}" }
                    input {
                        r#type: "number", min: "0",
                        value: "{date_uncertainty}",
                        oninput: move |e| date_uncertainty.set(e.value().parse().unwrap_or(50)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Stratigraphic Layer / Locus" }
                    input {
                        r#type: "text", placeholder: "e.g. Layer IIb, Locus 204…",
                        value: "{stratigraphy_layer}",
                        oninput: move |e| stratigraphy_layer.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Excavation / Contextual Notes" }
                textarea {
                    value: "{excavation_notes}",
                    oninput: move |e| excavation_notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); display: flex; gap: 16px; flex-wrap: wrap;",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{period}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{site_type}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-accent); font-weight: bold;", "{date_bp} ±{date_uncertainty} BP" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{dating_method}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); width: 100%;", "QualiaDB → Allen Interval Algebra | stratigraphic graph | dendrochronology sieve" }
            }
        }
    }
}
