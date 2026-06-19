use dioxus::prelude::*;

#[component]
pub fn EcoQueerTheoryQapp() -> Element {
    let mut theoretical_fusion = use_signal(|| "Queer Ecology".to_string());
    let mut theorist = use_signal(|| "Mortimer-Sandilands".to_string());
    let mut nature_phenomenon = use_signal(|| "Animal Sexuality".to_string());
    let mut queer_modality = use_signal(|| "Temporal".to_string());
    let mut normativity_critique = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let theoretical_fusions = [
        "Queer Ecology",
        "Queering Nature",
        "Trans-Species Kinship",
        "Queer Phenomenology of Landscape",
    ];
    let theorists = ["Mortimer-Sandilands", "Erickson", "Alaimo", "Gaard"];
    let nature_phenomena = [
        "Animal Sexuality",
        "Plant Reproduction",
        "Seasonal Change",
        "Symbiosis",
        "Decomposition",
    ];
    let queer_modalities = ["Temporal", "Spatial", "Corporeal", "Affective"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Eco-Queer Theory" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Fusion" }
                select {
                    value: "{theoretical_fusion}", onchange: move |e| theoretical_fusion.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in theoretical_fusions { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theorist" }
                select {
                    value: "{theorist}", onchange: move |e| theorist.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in theorists { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Nature Phenomenon" }
                select {
                    value: "{nature_phenomenon}", onchange: move |e| nature_phenomenon.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in nature_phenomena { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Queer Modality" }
                select {
                    value: "{queer_modality}", onchange: move |e| queer_modality.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in queer_modalities { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Normativity Critique: {normativity_critique}" }
                input { r#type: "range", min: "0", max: "100", value: "{normativity_critique}",
                    oninput: move |e| normativity_critique.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;" }
            }
            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea { value: "{notes}", oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;" }
            }
            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{theoretical_fusion} | {theorist} | {nature_phenomenon} | {queer_modality} | critique: {normativity_critique}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → eco-queer theory engine | discourse sieve | anchor" }
            }
        }
    }
}
