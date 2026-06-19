use dioxus::prelude::*;

#[component]
pub fn HistoryOfArtAndArchitectureQapp() -> Element {
    let mut epoch = use_signal(|| "Renaissance".to_string());
    let mut medium = use_signal(|| "Painting".to_string());
    let mut movement = use_signal(|| "Impressionism".to_string());
    let mut geographic_origin = use_signal(|| "Western Europe".to_string());
    let mut provenance_certainty = use_signal(|| 70u32);
    let mut notes = use_signal(|| String::new());

    let epochs = [
        "Prehistoric",
        "Ancient",
        "Medieval",
        "Renaissance",
        "Baroque",
        "Modern",
        "Contemporary",
    ];
    let mediums = [
        "Painting",
        "Sculpture",
        "Architecture",
        "Photography",
        "Installation",
        "Drawing",
        "Mixed",
    ];
    let movements = [
        "Impressionism",
        "Cubism",
        "Surrealism",
        "Abstract Expressionism",
        "Pop Art",
        "Minimalism",
        "Postmodernism",
    ];
    let origins = [
        "Western Europe",
        "Eastern Europe",
        "Americas",
        "Asia",
        "Africa",
        "Oceania",
        "Middle East",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "History of Art & Architecture" }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Epoch" }
                select {
                    value: "{epoch}",
                    onchange: move |e| epoch.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in epochs { option { value: "{x}", "{x}" } }
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
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Movement" }
                select {
                    value: "{movement}",
                    onchange: move |e| movement.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in movements { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Geographic Origin" }
                select {
                    value: "{geographic_origin}",
                    onchange: move |e| geographic_origin.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in origins { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Provenance Certainty: {provenance_certainty}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{provenance_certainty}",
                    oninput: move |e| provenance_certainty.set(e.value().parse().unwrap_or(70)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{epoch} | {medium} | {movement} | {geographic_origin} | Provenance: {provenance_certainty}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
