use dioxus::prelude::*;

#[component]
pub fn WhiteStudiesQapp() -> Element {
    let mut analytical_focus = use_signal(|| "Whiteness as Invisible Norm".to_string());
    let mut theoretical_tradition = use_signal(|| "Critical Whiteness Studies".to_string());
    let mut context = use_signal(|| "US".to_string());
    let mut key_theorist = use_signal(|| "Roediger".to_string());
    let mut privilege_index = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let foci = [
        "Whiteness as Invisible Norm",
        "White Privilege",
        "White Fragility",
        "White Nationalism",
        "Historical Construction of Whiteness",
        "Economic Whiteness",
    ];
    let traditions = [
        "Critical Whiteness Studies",
        "CRT",
        "Labour Whiteness Studies",
        "Abolitionism",
    ];
    let contexts = [
        "US",
        "UK",
        "Australia",
        "Europe",
        "South Africa",
        "Brazil",
        "Global",
    ];
    let theorists = [
        "Roediger",
        "Frankenberg",
        "DiAngelo",
        "Morrison",
        "Ignatiev",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "White Studies" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Analytical Focus" }
                select {
                    value: "{analytical_focus}",
                    onchange: move |e| analytical_focus.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in foci { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Tradition" }
                select {
                    value: "{theoretical_tradition}",
                    onchange: move |e| theoretical_tradition.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in traditions { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Context" }
                select {
                    value: "{context}",
                    onchange: move |e| context.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in contexts { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Key Theorist" }
                select {
                    value: "{key_theorist}",
                    onchange: move |e| key_theorist.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in theorists { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Privilege Index: {privilege_index}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{privilege_index}",
                    oninput: move |e| privilege_index.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{analytical_focus} | {theoretical_tradition} | {context} | {key_theorist} | Privilege: {privilege_index}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → white studies engine | norm sieve | privilege anchor" }
            }
        }
    }
}
