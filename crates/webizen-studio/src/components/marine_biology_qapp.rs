use dioxus::prelude::*;

#[component]
pub fn MarineBiologyQapp() -> Element {
    let mut ecosystem = use_signal(|| "Coral Reef".to_string());
    let mut taxonomic_group = use_signal(|| "Fish".to_string());
    let mut depth_m = use_signal(|| 200u32);
    let mut temperature_c = use_signal(|| 15.0f64);
    let mut salinity_ppt = use_signal(|| 35.0f64);
    let mut ph = use_signal(|| 8.1f64);
    let dissolved_o2_mgl = use_signal(|| 7.0f64);
    let mut notes = use_signal(|| String::new());

    let ecosystems = [
        "Coral Reef",
        "Deep Sea",
        "Open Ocean",
        "Kelp Forest",
        "Mangrove",
        "Estuarine",
        "Hydrothermal Vent",
        "Polar Marine",
        "Intertidal Zone",
        "Seagrass Meadow",
    ];
    let taxonomic_groups = [
        "Fish",
        "Cephalopod",
        "Cetacean",
        "Shark",
        "Coral",
        "Plankton",
        "Benthic Invertebrate",
        "Algae",
        "Marine Mammal",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Marine Biology" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Ecosystem" }
                    select {
                        value: "{ecosystem}",
                        onchange: move |e| ecosystem.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in ecosystems { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Taxonomic Group" }
                    select {
                        value: "{taxonomic_group}",
                        onchange: move |e| taxonomic_group.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in taxonomic_groups { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Depth (m): {depth_m}" }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "11000",
                        value: "{depth_m}",
                        oninput: move |e| depth_m.set(e.value().parse().unwrap_or(200)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temperature (°C): {temperature_c:.1}" }
                    input {
                        r#type: "range",
                        min: "-2",
                        max: "35",
                        step: "1",
                        value: "{temperature_c()}",
                        oninput: move |e| temperature_c.set(e.value().parse().unwrap_or(15.0)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Salinity (ppt): {salinity_ppt:.1}" }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "50",
                        step: "1",
                        value: "{salinity_ppt()}",
                        oninput: move |e| salinity_ppt.set(e.value().parse().unwrap_or(35.0)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "pH: {ph:.2}" }
                    input {
                        r#type: "range",
                        min: "750",
                        max: "830",
                        value: "{ph() * 100.0}",
                        oninput: move |e| ph.set(e.value().parse::<f64>().unwrap_or(810.0) / 100.0),
                        style: "width: 100%; margin-top: 4px;"
                    }
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{ecosystem} | {taxonomic_group} | {depth_m}m | {temperature_c:.1}°C | pH{ph:.2} | Sal:{salinity_ppt:.0}ppt | O2:{dissolved_o2_mgl:.1}mg/L" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → marine biogeography engine | oceanographic sieve | taxonomic anchor" }
            }
        }
    }
}
