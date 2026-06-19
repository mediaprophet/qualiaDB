use dioxus::prelude::*;

#[component]
pub fn ArtHistoryQapp() -> Element {
    let mut period = use_signal(|| "Renaissance".to_string());
    let mut attribution_certainty = use_signal(|| 75u32);
    let mut iconographic_query = use_signal(|| String::new());
    let mut movement_start = use_signal(|| 1400i32);
    let mut movement_end = use_signal(|| 1600i32);
    let mut style_tag = use_signal(|| "Chiaroscuro".to_string());

    let periods = [
        "Ancient",
        "Medieval",
        "Renaissance",
        "Baroque",
        "Neoclassicism",
        "Romanticism",
        "Impressionism",
        "Modernism",
        "Contemporary",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Art History" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Art Period" }
                    select {
                        value: "{period}",
                        onchange: move |e| period.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for p in periods {
                            option { value: "{p}", "{p}" }
                        }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Style / Technique" }
                    input {
                        r#type: "text",
                        value: "{style_tag}",
                        oninput: move |e| style_tag.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Iconographic Query (QualiaDB Semantic Search)" }
                input {
                    r#type: "text",
                    placeholder: "e.g. Madonna and Child, vanitas symbols, heroic nude…",
                    value: "{iconographic_query}",
                    oninput: move |e| iconographic_query.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Movement Start (CE)" }
                    input {
                        r#type: "number",
                        value: "{movement_start}",
                        oninput: move |e| movement_start.set(e.value().parse().unwrap_or(1400)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Movement End (CE)" }
                    input {
                        r#type: "number",
                        value: "{movement_end}",
                        oninput: move |e| movement_end.set(e.value().parse().unwrap_or(1600)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Attribution Certainty: {attribution_certainty}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{attribution_certainty}",
                    oninput: move |e| attribution_certainty.set(e.value().parse().unwrap_or(75)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Allen Interval — Movement Span" }
                div {
                    style: "font-size: 0.85rem; color: var(--qualia-text-muted); margin-bottom: 8px;",
                    "Period: {period} | Duration: {movement_end() - movement_start()} years"
                }
                div {
                    style: "position: relative; height: 24px; background: var(--qualia-bg); border-radius: 4px; overflow: hidden;",
                    div {
                        style: "position: absolute; top: 0; left: 5%; right: 5%; height: 100%; background: linear-gradient(90deg, var(--qualia-accent), var(--qualia-accent)); border-radius: 4px; display: flex; align-items: center; justify-content: center; font-size: 0.75rem; color: var(--qualia-surface); font-weight: bold;",
                        "{period}: {movement_start}–{movement_end}"
                    }
                }
                div {
                    style: "margin-top: 12px; font-size: 0.8rem; color: var(--qualia-text-muted);",
                    "Attribution confidence (epistemic): {attribution_certainty}% | Style: {style_tag}"
                }
            }
        }
    }
}
