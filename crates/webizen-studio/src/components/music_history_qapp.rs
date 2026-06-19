use dioxus::prelude::*;

#[component]
pub fn MusicHistoryQapp() -> Element {
    let mut era = use_signal(|| "Baroque".to_string());
    let mut composer = use_signal(|| String::new());
    let mut work = use_signal(|| String::new());
    let mut geographic_origin = use_signal(|| String::new());
    let mut style_characteristic = use_signal(|| String::new());
    let mut year = use_signal(|| 1700i32);

    let eras = [
        ("Medieval", "500–1400"),
        ("Renaissance", "1400–1600"),
        ("Baroque", "1600–1750"),
        ("Classical", "1750–1820"),
        ("Romantic", "1820–1900"),
        ("Late Romantic / Impressionism", "1875–1920"),
        ("Modernism", "1900–1945"),
        ("Post-War / Avant-Garde", "1945–1975"),
        ("Minimalism", "1960–"),
        ("Contemporary / Post-Modern", "1975–"),
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Music History" }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Historical Era" }
                select {
                    value: "{era}",
                    onchange: move |e| era.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                    for (name, dates) in eras {
                        option { value: "{name}", "{name} ({dates})" }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Composer" }
                    input {
                        r#type: "text", placeholder: "e.g. J.S. Bach, Clara Schumann…",
                        value: "{composer}",
                        oninput: move |e| composer.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Work / Opus" }
                    input {
                        r#type: "text", placeholder: "e.g. Mass in B minor, BWV 232…",
                        value: "{work}",
                        oninput: move |e| work.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Geographic / Court Origin" }
                    input {
                        r#type: "text", placeholder: "e.g. Leipzig, Vienna, Paris, Versailles…",
                        value: "{geographic_origin}",
                        oninput: move |e| geographic_origin.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Style Characteristic" }
                    input {
                        r#type: "text", placeholder: "e.g. basso continuo, Gesamtkunstwerk…",
                        value: "{style_characteristic}",
                        oninput: move |e| style_characteristic.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Year / Approx. Date: {year}" }
                input {
                    r#type: "range", min: "500", max: "2026",
                    value: "{year}",
                    oninput: move |e| year.set(e.value().parse().unwrap_or(1700)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Allen Interval — Historical Placement" }
                div {
                    style: "display: flex; align-items: center; gap: 12px; flex-wrap: wrap;",
                    span {
                        style: "background: var(--qualia-border); padding: 4px 10px; border-radius: 4px; font-size: 0.85rem;",
                        "Era: {era}"
                    }
                    span {
                        style: "background: var(--qualia-border); padding: 4px 10px; border-radius: 4px; font-size: 0.85rem;",
                        "Year: {year}"
                    }
                    if !geographic_origin().is_empty() {
                        span {
                            style: "background: var(--qualia-border); padding: 4px 10px; border-radius: 4px; font-size: 0.85rem;",
                            "{geographic_origin}"
                        }
                    }
                }
                if !composer().is_empty() {
                    div {
                        style: "margin-top: 8px; font-size: 0.85rem; color: var(--qualia-text);",
                        "{composer}"
                        if !work().is_empty() { " — {work}" }
                    }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → Allen Interval engine | musicological knowledge graph" }
            }
        }
    }
}
