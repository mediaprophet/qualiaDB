use dioxus::prelude::*;

#[component]
pub fn SpinozaStudiesQapp() -> Element {
    let mut spinozist_concept = use_signal(|| "Conatus".to_string());
    let mut reading_tradition = use_signal(|| "Radical Democratic".to_string());
    let mut interlocutor = use_signal(|| "Deleuze".to_string());
    let mut text = use_signal(|| "Ethics".to_string());
    let mut interpretive_freedom = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let concepts = [
        "Conatus",
        "Substance-God-Nature",
        "Modes and Attributes",
        "Affect",
        "Adequate Idea",
        "Multitude",
        "Immanence",
    ];
    let traditions = [
        "Enlightenment",
        "Radical Democratic",
        "Marxist-Spinozist",
        "Deleuzian",
        "Negri's Multitude",
        "Feminist Spinozism",
    ];
    let interlocutors = [
        "Descartes",
        "Leibniz",
        "Hegel",
        "Marx",
        "Nietzsche",
        "Deleuze",
        "Negri",
    ];
    let texts = [
        "Ethics",
        "Theological-Political Treatise",
        "Political Treatise",
        "Letters",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Spinoza Studies" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Spinozist Concept" }
                select {
                    value: "{spinozist_concept}",
                    onchange: move |e| spinozist_concept.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in concepts { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Reading Tradition" }
                select {
                    value: "{reading_tradition}",
                    onchange: move |e| reading_tradition.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in traditions { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Interlocutor" }
                select {
                    value: "{interlocutor}",
                    onchange: move |e| interlocutor.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in interlocutors { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Text" }
                select {
                    value: "{text}",
                    onchange: move |e| text.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in texts { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Interpretive Freedom: {interpretive_freedom}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{interpretive_freedom}",
                    oninput: move |e| interpretive_freedom.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{spinozist_concept} | {reading_tradition} | {text} | Freedom: {interpretive_freedom}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → Spinoza studies engine | conatus sieve | immanence anchor" }
            }
        }
    }
}
