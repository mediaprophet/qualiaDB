use dioxus::prelude::*;

#[component]
pub fn UrbanEcologyQapp() -> Element {
    let mut ecosystem_service = use_signal(|| "Heat Island Mitigation".to_string());
    let mut habitat_type = use_signal(|| "Urban Forest".to_string());
    let mut species_guild = use_signal(|| "Bird".to_string());
    let mut impervious_surface = use_signal(|| 60u32);
    let mut green_space = use_signal(|| 25u32);
    let mut species_richness = use_signal(|| 80u32);
    let mut notes = use_signal(|| String::new());

    let ecosystem_services = [
        "Heat Island Mitigation",
        "Air Purification",
        "Stormwater Management",
        "Biodiversity",
        "Noise Reduction",
        "Carbon Sequestration",
    ];
    let habitat_types = [
        "Urban Forest",
        "Green Roof",
        "Wetland",
        "Street Tree",
        "Park",
        "Urban Farm",
        "Brownfield",
    ];
    let species_guilds = ["Bird", "Insect", "Mammal", "Plant", "Fungi", "Aquatic"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 {
                style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;",
                "Urban Ecology"
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Ecosystem Service" }
                select {
                    value: "{ecosystem_service}",
                    onchange: move |e| ecosystem_service.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in ecosystem_services { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Habitat Type" }
                select {
                    value: "{habitat_type}",
                    onchange: move |e| habitat_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in habitat_types { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Species Guild" }
                select {
                    value: "{species_guild}",
                    onchange: move |e| species_guild.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in species_guilds { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Impervious Surface: {impervious_surface}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{impervious_surface}",
                    oninput: move |e| impervious_surface.set(e.value().parse().unwrap_or(60)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Green Space: {green_space}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{green_space}",
                    oninput: move |e| green_space.set(e.value().parse().unwrap_or(25)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Species Richness: {species_richness}" }
                input {
                    r#type: "range", min: "0", max: "500",
                    value: "{species_richness}",
                    oninput: move |e| species_richness.set(e.value().parse().unwrap_or(80)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{ecosystem_service} | {habitat_type} | {species_guild} | imp {impervious_surface}% | green {green_space}% | spp {species_richness}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
