use dioxus::prelude::*;

#[component]
pub fn ArchitecturalHistoryQapp() -> Element {
    let mut period = use_signal(|| "Ancient".to_string());
    let mut style = use_signal(|| "Gothic".to_string());
    let mut region = use_signal(|| "Europe".to_string());
    let mut structural_material = use_signal(|| "Stone".to_string());
    let mut year_ce = use_signal(|| 1200i32);
    let mut notes = use_signal(|| String::new());

    let periods = [
        "Ancient",
        "Medieval",
        "Renaissance",
        "Baroque",
        "Neoclassical",
        "Industrial",
        "Modernist",
        "Postmodern",
        "Contemporary",
    ];
    let styles = [
        "Gothic",
        "Palladian",
        "Brutalist",
        "Deconstructivist",
        "Vernacular",
        "Art Nouveau",
        "International",
    ];
    let regions = [
        "Europe",
        "Americas",
        "Asia",
        "Middle East",
        "Africa",
        "Oceania",
    ];
    let materials = [
        "Stone",
        "Brick",
        "Timber",
        "Iron/Steel",
        "Concrete",
        "Glass",
        "Mixed",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Architectural History" }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Period" }
                select {
                    value: "{period}",
                    onchange: move |e| period.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in periods { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Architectural Style" }
                select {
                    value: "{style}",
                    onchange: move |e| style.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in styles { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Region" }
                select {
                    value: "{region}",
                    onchange: move |e| region.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in regions { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Structural Material" }
                select {
                    value: "{structural_material}",
                    onchange: move |e| structural_material.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in materials { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Year CE: {year_ce}" }
                input {
                    r#type: "range", min: "-500", max: "2025",
                    value: "{year_ce}",
                    oninput: move |e| year_ce.set(e.value().parse().unwrap_or(1200)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{period} | {style} | {region} | {structural_material} | {year_ce} CE" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
