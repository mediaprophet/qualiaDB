use dioxus::prelude::*;

#[component]
pub fn AncientNearEasternStudiesQapp() -> Element {
    let mut culture = use_signal(|| "Sumerian".to_string());
    let mut script = use_signal(|| "Cuneiform".to_string());
    let mut period_bce = use_signal(|| 2500u32);
    let mut source_type = use_signal(|| "Administrative Tablet".to_string());
    let mut site = use_signal(|| String::new());
    let mut discipline = use_signal(|| "Philology".to_string());
    let mut notes = use_signal(|| String::new());

    let cultures = [
        "Sumerian",
        "Akkadian",
        "Babylonian",
        "Assyrian",
        "Hittite",
        "Ugaritic",
        "Phoenician",
        "Aramaean",
        "Persian Achaemenid",
    ];
    let scripts = [
        "Cuneiform",
        "Linear A",
        "Linear B",
        "Proto-Sinaitic",
        "Alphabetic",
    ];
    let source_types = [
        "Administrative Tablet",
        "Literary Text",
        "Royal Inscription",
        "Legal Document",
        "Divination Text",
    ];
    let disciplines = ["Philology", "Archaeology", "History", "Religion"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Ancient Near Eastern Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Culture" }
                    select {
                        value: "{culture}",
                        onchange: move |e| culture.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in cultures { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Script" }
                    select {
                        value: "{script}",
                        onchange: move |e| script.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in scripts { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Source Type" }
                    select {
                        value: "{source_type}",
                        onchange: move |e| source_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in source_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Discipline" }
                    select {
                        value: "{discipline}",
                        onchange: move |e| discipline.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in disciplines { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Period BCE: {period_bce}" }
                input {
                    r#type: "range",
                    min: "330",
                    max: "3500",
                    value: "{period_bce}",
                    oninput: move |e| period_bce.set(e.value().parse().unwrap_or(2500)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Archaeological Site" }
                input {
                    r#type: "text",
                    value: "{site}",
                    oninput: move |e| site.set(e.value()),
                    placeholder: "e.g. Ur, Nineveh, Ugarit",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    value: "{notes}",
                    oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 80px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{culture} | {script} | {period_bce} BCE | {source_type} | {discipline}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → philological engine | cuneiform corpus | temporal sieve" }
            }
        }
    }
}
