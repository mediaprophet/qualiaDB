use dioxus::prelude::*;

#[component]
pub fn MaritimeHistoryQapp() -> Element {
    let mut era = use_signal(|| "Age of Exploration".to_string());
    let mut vessel_type = use_signal(|| "Carrack".to_string());
    let mut trade_route = use_signal(|| "Spice Route".to_string());
    let mut cargo_type = use_signal(|| "Spices".to_string());
    let mut tonnage = use_signal(|| 5000u32);
    let mut notes = use_signal(|| String::new());

    let eras = [
        "Ancient",
        "Medieval",
        "Age of Exploration",
        "Colonial",
        "Industrial",
        "Modern",
    ];
    let vessels = [
        "Galley",
        "Carrack",
        "Frigate",
        "Clipper",
        "Steamship",
        "Submarine",
        "Container Ship",
    ];
    let routes = [
        "Silk Road Maritime",
        "Atlantic Triangle",
        "Spice Route",
        "Arctic",
        "Pacific",
        "Indian Ocean",
    ];
    let cargos = [
        "Spices",
        "Slaves",
        "Grain",
        "Manufactured Goods",
        "Petroleum",
        "Containers",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Maritime History" }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Era" }
                select {
                    value: "{era}",
                    onchange: move |e| era.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in eras { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Vessel Type" }
                select {
                    value: "{vessel_type}",
                    onchange: move |e| vessel_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in vessels { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Trade Route" }
                select {
                    value: "{trade_route}",
                    onchange: move |e| trade_route.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in routes { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Cargo Type" }
                select {
                    value: "{cargo_type}",
                    onchange: move |e| cargo_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in cargos { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tonnage: {tonnage}" }
                input {
                    r#type: "range", min: "0", max: "300000",
                    value: "{tonnage}",
                    oninput: move |e| tonnage.set(e.value().parse().unwrap_or(5000)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{era} | {vessel_type} | {trade_route} | {cargo_type} | {tonnage}t" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
