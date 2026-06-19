use dioxus::prelude::*;

#[component]
pub fn AfricanStudiesQapp() -> Element {
    let mut subregion = use_signal(|| "West Africa".to_string());
    let mut period = use_signal(|| "Contemporary".to_string());
    let mut disciplinary_lens = use_signal(|| "History".to_string());
    let mut language_group = use_signal(|| "Bantu".to_string());
    let mut theoretical_tradition = use_signal(|| "Ubuntu".to_string());
    let mut notes = use_signal(|| String::new());

    let subregions = [
        "West Africa",
        "East Africa",
        "Central Africa",
        "Southern Africa",
        "North Africa",
        "Horn of Africa",
        "Sahel",
        "Great Lakes",
        "Island States",
    ];
    let periods = [
        "Pre-Colonial",
        "Colonial",
        "Independence",
        "Cold War",
        "Structural Adjustment",
        "Contemporary",
    ];
    let lenses = [
        "History",
        "Political Economy",
        "Literature",
        "Anthropology",
        "African Philosophy",
        "Pan-Africanism",
        "Postcolonial",
        "Decolonial",
    ];
    let lang_groups = [
        "Bantu",
        "Cushitic",
        "Afro-Asiatic",
        "Nilo-Saharan",
        "Khoisan",
        "Pidgin-Creole",
    ];
    let traditions = [
        "Ubuntu",
        "Négritude",
        "Afrocentrism",
        "Third-Worldism",
        "Liberal",
        "Marxist",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "African Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Subregion" }
                    select {
                        value: "{subregion}",
                        onchange: move |e| subregion.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in subregions { option { value: "{x}", "{x}" } }
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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Disciplinary Lens" }
                    select {
                        value: "{disciplinary_lens}",
                        onchange: move |e| disciplinary_lens.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in lenses { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Language Group" }
                    select {
                        value: "{language_group}",
                        onchange: move |e| language_group.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in lang_groups { option { value: "{x}", "{x}" } }
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{subregion} | {period} | {disciplinary_lens} | {theoretical_tradition}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → decolonial knowledge engine | provenance graph | epistemic sieve" }
            }
        }
    }
}
