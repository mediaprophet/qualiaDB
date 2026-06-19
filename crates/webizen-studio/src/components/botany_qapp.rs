use dioxus::prelude::*;

#[component]
pub fn BotanyQapp() -> Element {
    let mut plant_division = use_signal(|| "Angiosperms".to_string());
    let mut growth_form = use_signal(|| "Tree".to_string());
    let mut habitat = use_signal(|| "Tropical Rainforest".to_string());
    let mut phenological_stage = use_signal(|| "Vegetative".to_string());
    let mut pollination_mechanism = use_signal(|| "Entomophily (insect)".to_string());
    let mut specimen_notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Botany QApp" }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Plant Division" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| plant_division.set(e.value()),
                        option { "Angiosperms" }
                        option { "Gymnosperms" }
                        option { "Ferns (Pteridophyta)" }
                        option { "Mosses (Bryophyta)" }
                        option { "Algae (Chlorophyta)" }
                        option { "Fungi (Basidiomycota)" }
                        option { "Fungi (Ascomycota)" }
                        option { "Lichens" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Growth Form" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| growth_form.set(e.value()),
                        option { "Tree" }
                        option { "Shrub" }
                        option { "Herb" }
                        option { "Vine" }
                        option { "Epiphyte" }
                        option { "Aquatic" }
                        option { "Succulent" }
                        option { "Geophyte" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Habitat" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| habitat.set(e.value()),
                        option { "Tropical Rainforest" }
                        option { "Temperate Forest" }
                        option { "Desert" }
                        option { "Grassland" }
                        option { "Wetland" }
                        option { "Arctic Tundra" }
                        option { "Alpine" }
                        option { "Mediterranean Shrubland" }
                        option { "Mangrove" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Phenological Stage" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| phenological_stage.set(e.value()),
                        option { "Germination" }
                        option { "Vegetative" }
                        option { "Flowering" }
                        option { "Fruiting" }
                        option { "Senescence" }
                        option { "Dormancy" }
                        option { "Leaf Flush" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Pollination Mechanism" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| pollination_mechanism.set(e.value()),
                        option { "Entomophily (insect)" }
                        option { "Anemophily (wind)" }
                        option { "Ornithophily (bird)" }
                        option { "Chiropterophily (bat)" }
                        option { "Hydrophily (water)" }
                        option { "Self-pollination" }
                        option { "Apomixis" }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Specimen Notes" }
                textarea {
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; min-height: 70px; box-sizing: border-box; resize: vertical;",
                    placeholder: "Taxonomic identification, GPS coordinates, voucher number, morphological description...",
                    oninput: move |e| specimen_notes.set(e.value()),
                    "{specimen_notes}"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Output / Analysis" }
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 0.8rem;",
                    div { style: "color: var(--qualia-text-muted);", "Division:" }
                    div { style: "color: var(--qualia-text);", "{plant_division}" }
                    div { style: "color: var(--qualia-text-muted);", "Growth Form:" }
                    div { style: "color: var(--qualia-text);", "{growth_form}" }
                    div { style: "color: var(--qualia-text-muted);", "Habitat:" }
                    div { style: "color: var(--qualia-text);", "{habitat}" }
                    div { style: "color: var(--qualia-text-muted);", "Phenophase:" }
                    div { style: "color: var(--qualia-text);", "{phenological_stage}" }
                    div { style: "color: var(--qualia-text-muted);", "Pollination:" }
                    div { style: "color: var(--qualia-text);", "{pollination_mechanism}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 12px; border-top: 1px solid var(--qualia-border); padding-top: 8px;",
                    "QualiaDB → Allen Interval phenology engine | taxonomy graph | ecological sieve"
                }
            }
        }
    }
}
