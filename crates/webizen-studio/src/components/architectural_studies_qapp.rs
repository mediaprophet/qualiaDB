use dioxus::prelude::*;

#[component]
pub fn ArchitecturalStudiesQapp() -> Element {
    let mut design_approach = use_signal(|| "Parametric".to_string());
    let mut scale = use_signal(|| "Building".to_string());
    let mut program = use_signal(|| "Residential".to_string());
    let mut structural_system = use_signal(|| "Frame".to_string());
    let mut floor_area = use_signal(|| 500u32);
    let mut sustainability_rating = use_signal(|| 60u32);
    let mut notes = use_signal(|| String::new());

    let design_approaches = [
        "Parametric",
        "Sustainable",
        "Universal Design",
        "Contextual",
        "Typological",
        "Phenomenological",
    ];
    let scales = [
        "Interior",
        "Building",
        "Campus",
        "Urban Block",
        "City",
        "Regional",
    ];
    let programs = [
        "Residential",
        "Commercial",
        "Cultural",
        "Civic",
        "Industrial",
        "Mixed-Use",
        "Landscape",
    ];
    let structural_systems = ["Bearing Wall", "Frame", "Shell", "Tensile", "Hybrid"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 {
                style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;",
                "Architectural Studies"
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Design Approach" }
                select {
                    value: "{design_approach}",
                    onchange: move |e| design_approach.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in design_approaches { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Scale" }
                select {
                    value: "{scale}",
                    onchange: move |e| scale.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in scales { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Program" }
                select {
                    value: "{program}",
                    onchange: move |e| program.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in programs { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Structural System" }
                select {
                    value: "{structural_system}",
                    onchange: move |e| structural_system.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in structural_systems { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Floor Area (m²): {floor_area}" }
                input {
                    r#type: "range", min: "0", max: "500000",
                    value: "{floor_area}",
                    oninput: move |e| floor_area.set(e.value().parse().unwrap_or(500)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sustainability Rating: {sustainability_rating}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{sustainability_rating}",
                    oninput: move |e| sustainability_rating.set(e.value().parse().unwrap_or(60)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{design_approach} | {scale} | {program} | {structural_system} | {floor_area} m²" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
