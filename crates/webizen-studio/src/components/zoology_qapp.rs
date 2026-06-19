use dioxus::prelude::*;

#[component]
pub fn ZoologyQapp() -> Element {
    let mut animal_class = use_signal(|| "Mammalia".to_string());
    let mut habitat = use_signal(|| "Terrestrial".to_string());
    let mut diet_type = use_signal(|| "Omnivore".to_string());
    let mut locomotion = use_signal(|| "Quadrupedal".to_string());
    let mut social_structure = use_signal(|| "Small Group".to_string());
    let mut specimen_notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Zoology QApp" }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Animal Class" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| animal_class.set(e.value()),
                        option { "Mammalia" }
                        option { "Aves" }
                        option { "Reptilia" }
                        option { "Amphibia" }
                        option { "Actinopterygii" }
                        option { "Chondrichthyes" }
                        option { "Insecta" }
                        option { "Arachnida" }
                        option { "Crustacea" }
                        option { "Mollusca" }
                        option { "Annelida" }
                        option { "Echinodermata" }
                        option { "Cephalochordata" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Habitat" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| habitat.set(e.value()),
                        option { "Terrestrial" }
                        option { "Freshwater" }
                        option { "Marine" }
                        option { "Arboreal" }
                        option { "Fossorial" }
                        option { "Aerial" }
                        option { "Semi-aquatic" }
                        option { "Cave / Subterranean" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Diet Type" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| diet_type.set(e.value()),
                        option { "Herbivore" }
                        option { "Carnivore" }
                        option { "Omnivore" }
                        option { "Insectivore" }
                        option { "Piscivore" }
                        option { "Filter Feeder" }
                        option { "Detritivore" }
                        option { "Nectarivore" }
                        option { "Frugivore" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Locomotion" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| locomotion.set(e.value()),
                        option { "Quadrupedal" }
                        option { "Bipedal" }
                        option { "Flight" }
                        option { "Aquatic Swimming" }
                        option { "Brachiation" }
                        option { "Limbless / Serpentine" }
                        option { "Jumping / Saltatorial" }
                        option { "Burrowing" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Social Structure" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| social_structure.set(e.value()),
                        option { "Solitary" }
                        option { "Pair" }
                        option { "Small Group" }
                        option { "Herd" }
                        option { "Colony" }
                        option { "Superorganism" }
                        option { "Fission-fusion" }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Specimen Notes" }
                textarea {
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; min-height: 70px; box-sizing: border-box; resize: vertical;",
                    placeholder: "Species identification, conservation status, field observations, morphometrics...",
                    oninput: move |e| specimen_notes.set(e.value()),
                    "{specimen_notes}"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Output / Analysis" }
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 0.8rem;",
                    div { style: "color: var(--qualia-text-muted);", "Class:" }
                    div { style: "color: var(--qualia-text);", "{animal_class}" }
                    div { style: "color: var(--qualia-text-muted);", "Habitat:" }
                    div { style: "color: var(--qualia-text);", "{habitat}" }
                    div { style: "color: var(--qualia-text-muted);", "Diet:" }
                    div { style: "color: var(--qualia-text);", "{diet_type}" }
                    div { style: "color: var(--qualia-text-muted);", "Locomotion:" }
                    div { style: "color: var(--qualia-text);", "{locomotion}" }
                    div { style: "color: var(--qualia-text-muted);", "Social:" }
                    div { style: "color: var(--qualia-text);", "{social_structure}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 12px; border-top: 1px solid var(--qualia-border); padding-top: 8px;",
                    "QualiaDB → taxonomy graph | Allen Interval phenology | ecological sieve"
                }
            }
        }
    }
}
