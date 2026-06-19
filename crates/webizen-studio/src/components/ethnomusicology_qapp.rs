use dioxus::prelude::*;

#[component]
pub fn EthnomusicologyQapp() -> Element {
    let mut tradition = use_signal(|| "West African".to_string());
    let mut method = use_signal(|| "Fieldwork".to_string());
    let mut cultural_context = use_signal(|| String::new());
    let mut instrument_family = use_signal(|| "Membranophone".to_string());
    let mut scale_or_mode = use_signal(|| String::new());
    let mut trance_or_ritual_function = use_signal(|| "None".to_string());
    let mut notes = use_signal(|| String::new());

    let traditions = [
        "West African",
        "South Asian Classical",
        "Arabic Maqam",
        "Latin American",
        "Indigenous American",
        "Balkan",
        "East Asian",
        "Afro-Caribbean",
        "Electronic",
        "Hip-Hop as Folk",
    ];
    let methods = [
        "Fieldwork",
        "Archive",
        "Oral History",
        "Transcription",
        "Audio Analysis",
        "Network Analysis",
    ];
    let families = [
        "Chordophone",
        "Aerophone",
        "Membranophone",
        "Idiophone",
        "Electrophone",
    ];
    let ritual_functions = [
        "None",
        "Trance Induction",
        "Healing",
        "Funeral",
        "Initiation",
        "Seasonal Ritual",
        "War/Victory",
        "Devotional",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Ethnomusicology" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tradition" }
                    select {
                        value: "{tradition}",
                        onchange: move |e| tradition.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in traditions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Method" }
                    select {
                        value: "{method}",
                        onchange: move |e| method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in methods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Instrument Family" }
                    select {
                        value: "{instrument_family}",
                        onchange: move |e| instrument_family.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in families { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Trance / Ritual Function" }
                    select {
                        value: "{trance_or_ritual_function}",
                        onchange: move |e| trance_or_ritual_function.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in ritual_functions { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Cultural Context" }
                    input {
                        r#type: "text",
                        value: "{cultural_context}",
                        oninput: move |e| cultural_context.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Scale / Mode" }
                    input {
                        r#type: "text",
                        value: "{scale_or_mode}",
                        oninput: move |e| scale_or_mode.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{tradition} | {method} | {instrument_family} | {trance_or_ritual_function}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → acoustic analysis engine | cultural context graph | oral tradition sieve" }
            }
        }
    }
}
