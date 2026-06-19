use dioxus::prelude::*;

#[component]
pub fn MaterialistAestheticsQapp() -> Element {
    let mut theoretical_strand = use_signal(|| "Marxist Aesthetics".to_string());
    let mut aesthetic_concept = use_signal(|| "Commodity Form".to_string());
    let mut medium = use_signal(|| "Literature".to_string());
    let mut production_mode = use_signal(|| "Artisanal".to_string());
    let mut value_type = use_signal(|| "Use".to_string());
    let mut notes = use_signal(|| String::new());

    let strands = [
        "Marxist Aesthetics",
        "Material Poetics",
        "New Materialism",
        "Object-Oriented Aesthetics",
        "Sensory Studies",
        "Political Economy of Art",
    ];
    let concepts = [
        "Commodity Form",
        "Aura",
        "Kitsch",
        "Taste",
        "Sublime",
        "Ugly",
        "Uncanny",
    ];
    let mediums = [
        "Literature",
        "Visual Art",
        "Music",
        "Architecture",
        "Digital",
        "Performance",
    ];
    let modes = ["Artisanal", "Industrial", "Post-Fordist", "Platform"];
    let values = ["Use", "Exchange", "Sign", "Aesthetic"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Materialist Aesthetics" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Strand" }
                select {
                    value: "{theoretical_strand}",
                    onchange: move |e| theoretical_strand.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in strands { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Aesthetic Concept" }
                select {
                    value: "{aesthetic_concept}",
                    onchange: move |e| aesthetic_concept.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in concepts { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Medium" }
                select {
                    value: "{medium}",
                    onchange: move |e| medium.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in mediums { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Production Mode" }
                select {
                    value: "{production_mode}",
                    onchange: move |e| production_mode.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in modes { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Value Type" }
                select {
                    value: "{value_type}",
                    onchange: move |e| value_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in values { option { value: "{x}", "{x}" } }
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{theoretical_strand} | {aesthetic_concept} | {medium} | {production_mode} | {value_type}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → materialist aesthetics engine | commodity sieve | value anchor" }
            }
        }
    }
}
