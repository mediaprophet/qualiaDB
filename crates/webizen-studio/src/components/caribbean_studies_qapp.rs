use dioxus::prelude::*;

#[component]
pub fn CaribbeanStudiesQapp() -> Element {
    let mut subregion = use_signal(|| "Greater Antilles".to_string());
    let mut disciplinary_lens = use_signal(|| "History".to_string());
    let mut colonial_heritage = use_signal(|| "British".to_string());
    let mut language = use_signal(|| "English".to_string());
    let mut theoretical_lens = use_signal(|| "Plantation Theory".to_string());
    let mut notes = use_signal(|| String::new());

    let subregions = [
        "Greater Antilles",
        "Lesser Antilles",
        "French Caribbean",
        "Dutch Caribbean",
        "Commonwealth Caribbean",
        "Cuba",
        "Haiti",
        "Dominican Republic",
    ];
    let lenses = [
        "History",
        "Literature (Créolité)",
        "Anthropology",
        "Political Economy",
        "Music Studies",
        "Postcolonial",
        "Diaspora",
    ];
    let colonial_heritages = [
        "British", "French", "Spanish", "Dutch", "American", "Danish", "Mixed",
    ];
    let languages = [
        "English",
        "French",
        "Haitian Creole",
        "Spanish",
        "Dutch",
        "Papiamento",
    ];
    let theoretical_lenses = [
        "Plantation Theory",
        "Glissant Créolité",
        "Négritude",
        "Rastafarianism",
        "Caribbean Feminism",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Caribbean Studies" }

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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Disciplinary Lens" }
                    select {
                        value: "{disciplinary_lens}",
                        onchange: move |e| disciplinary_lens.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in lenses { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Colonial Heritage" }
                    select {
                        value: "{colonial_heritage}",
                        onchange: move |e| colonial_heritage.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in colonial_heritages { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Language" }
                    select {
                        value: "{language}",
                        onchange: move |e| language.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in languages { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Lens" }
                    select {
                        value: "{theoretical_lens}",
                        onchange: move |e| theoretical_lens.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in theoretical_lenses { option { value: "{x}", "{x}" } }
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{subregion} | {colonial_heritage} | {language} | {theoretical_lens}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → postcolonial engine | creolisation sieve | diaspora anchor" }
            }
        }
    }
}
