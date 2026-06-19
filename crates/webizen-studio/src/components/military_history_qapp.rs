use dioxus::prelude::*;

#[component]
pub fn MilitaryHistoryQapp() -> Element {
    let mut era = use_signal(|| "WWII".to_string());
    let mut conflict_type = use_signal(|| "Land Battle".to_string());
    let mut theatre = use_signal(|| "Europe".to_string());
    let mut force_size = use_signal(|| "Division".to_string());
    let mut casualty_estimate = use_signal(|| 10000u32);
    let mut notes = use_signal(|| String::new());

    let eras = [
        "Ancient",
        "Medieval",
        "Early Modern",
        "Napoleonic",
        "Industrial",
        "WWI",
        "WWII",
        "Cold War",
        "Contemporary",
    ];
    let conflict_types = [
        "Land Battle",
        "Naval Battle",
        "Siege",
        "Guerrilla",
        "Air Campaign",
        "Cyber/Information",
    ];
    let theatres = [
        "Europe",
        "Asia",
        "Americas",
        "Africa",
        "Pacific",
        "Middle East",
        "Global",
    ];
    let force_sizes = [
        "Squad",
        "Platoon",
        "Battalion",
        "Division",
        "Army",
        "Coalition",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Military History" }

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
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Conflict Type" }
                select {
                    value: "{conflict_type}",
                    onchange: move |e| conflict_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in conflict_types { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theatre" }
                select {
                    value: "{theatre}",
                    onchange: move |e| theatre.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in theatres { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Force Size" }
                select {
                    value: "{force_size}",
                    onchange: move |e| force_size.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in force_sizes { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Casualty Estimate: {casualty_estimate}" }
                input {
                    r#type: "range", min: "0", max: "1000000",
                    value: "{casualty_estimate}",
                    oninput: move |e| casualty_estimate.set(e.value().parse().unwrap_or(10000)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{era} | {conflict_type} | {theatre} | {force_size} | Casualties: {casualty_estimate}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
