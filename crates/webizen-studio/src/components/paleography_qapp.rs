use dioxus::prelude::*;

#[component]
pub fn PaleographyQapp() -> Element {
    let mut script_tradition = use_signal(|| "Latin".to_string());
    let mut period = use_signal(|| "Medieval".to_string());
    let mut material = use_signal(|| "Parchment".to_string());
    let mut writing_instrument = use_signal(|| "Quill".to_string());
    let mut legibility = use_signal(|| 70u32);
    let mut dating_confidence = use_signal(|| 60u32);
    let mut notes = use_signal(|| String::new());

    let scripts = [
        "Latin",
        "Greek",
        "Arabic",
        "Hebrew",
        "Cyrillic",
        "Cuneiform",
        "Hieroglyphic",
        "CJK",
        "Indic",
    ];
    let periods = ["Ancient", "Late Antique", "Medieval", "Early Modern"];
    let materials = [
        "Papyrus",
        "Parchment",
        "Paper",
        "Stone",
        "Clay",
        "Wax",
        "Bark",
    ];
    let instruments = ["Reed Pen", "Quill", "Metal Stylus", "Brush"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Paleography" }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Script Tradition" }
                select {
                    value: "{script_tradition}",
                    onchange: move |e| script_tradition.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in scripts { option { value: "{x}", "{x}" } }
                }
            }

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
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Material" }
                select {
                    value: "{material}",
                    onchange: move |e| material.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in materials { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Writing Instrument" }
                select {
                    value: "{writing_instrument}",
                    onchange: move |e| writing_instrument.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in instruments { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Legibility: {legibility}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{legibility}",
                    oninput: move |e| legibility.set(e.value().parse().unwrap_or(70)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Dating Confidence: {dating_confidence}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{dating_confidence}",
                    oninput: move |e| dating_confidence.set(e.value().parse().unwrap_or(60)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{script_tradition} | {period} | {material} | {writing_instrument} | Leg: {legibility}% | Date: {dating_confidence}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
