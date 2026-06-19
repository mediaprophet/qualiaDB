use dioxus::prelude::*;

#[component]
pub fn StudioArtQapp() -> Element {
    let mut medium = use_signal(|| "Oil on Canvas".to_string());
    let mut color_theory = use_signal(|| "Complementary".to_string());
    let mut composition = use_signal(|| "Rule of Thirds".to_string());
    let mut scale_cm_w = use_signal(|| 100u32);
    let mut scale_cm_h = use_signal(|| 80u32);
    let mut conceptual_statement = use_signal(|| String::new());
    let mut material_notes = use_signal(|| String::new());

    let mediums = [
        "Oil on Canvas",
        "Watercolour",
        "Acrylic",
        "Gouache",
        "Fresco",
        "Encaustic",
        "Pastel",
        "Charcoal",
        "Graphite",
        "Ink / Sumi-e",
        "Digital Painting",
        "Mixed Media",
        "Collage",
        "Photography",
        "Screen Print",
        "Etching / Intaglio",
        "Lithography",
        "Sculpture — Stone",
        "Sculpture — Bronze Cast",
        "Sculpture — Wood",
        "Sculpture — Ceramic",
        "Installation Art",
        "Performance Art",
        "Video Art",
    ];
    let color_theories = [
        "Complementary",
        "Analogous",
        "Triadic",
        "Split-Complementary",
        "Tetradic",
        "Monochromatic",
        "Warm / Cool Contrast",
        "Simultaneous Contrast (Albers)",
    ];
    let compositions = [
        "Rule of Thirds",
        "Golden Ratio / Phi Grid",
        "Symmetrical Balance",
        "Asymmetrical Balance",
        "Radial Symmetry",
        "Gestalt Grouping",
        "S-Curve / Dynamic Diagonal",
        "Flat / All-Over",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Studio Art" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Medium" }
                    select {
                        value: "{medium}",
                        onchange: move |e| medium.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for m in mediums { option { value: "{m}", "{m}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Colour Theory" }
                    select {
                        value: "{color_theory}",
                        onchange: move |e| color_theory.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for c in color_theories { option { value: "{c}", "{c}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Composition Principle" }
                    select {
                        value: "{composition}",
                        onchange: move |e| composition.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for c in compositions { option { value: "{c}", "{c}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Width (cm): {scale_cm_w}" }
                    input {
                        r#type: "range", min: "10", max: "500",
                        value: "{scale_cm_w}",
                        oninput: move |e| scale_cm_w.set(e.value().parse().unwrap_or(100)),
                        style: "width: 100%; margin-top: 8px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Height (cm): {scale_cm_h}" }
                    input {
                        r#type: "range", min: "10", max: "500",
                        value: "{scale_cm_h}",
                        oninput: move |e| scale_cm_h.set(e.value().parse().unwrap_or(80)),
                        style: "width: 100%; margin-top: 8px;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Conceptual Statement / Artist Intention" }
                textarea {
                    value: "{conceptual_statement}",
                    oninput: move |e| conceptual_statement.set(e.value()),
                    rows: "3",
                    placeholder: "Describe the conceptual basis, influences, intended affect…",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: vertical; box-sizing: border-box;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Material & Technical Notes" }
                textarea {
                    value: "{material_notes}",
                    oninput: move |e| material_notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); display: flex; gap: 16px; flex-wrap: wrap;",
                span { style: "font-size: 0.8rem; color: var(--qualia-text); font-weight: bold;", "{medium}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{scale_cm_w}×{scale_cm_h} cm" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{color_theory}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{composition}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); width: 100%;", "QualiaDB → visual ontology | material provenance graph" }
            }
        }
    }
}
