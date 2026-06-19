use dioxus::prelude::*;

#[component]
pub fn CentralAsianStudiesQapp() -> Element {
    let mut country = use_signal(|| "Kazakhstan".to_string());
    let mut period = use_signal(|| "Post-Soviet".to_string());
    let mut disciplinary_lens = use_signal(|| "History".to_string());
    let mut language_family = use_signal(|| "Turkic".to_string());
    let mut nomadic_vs_settled = use_signal(|| "Mixed".to_string());
    let mut notes = use_signal(|| String::new());

    let countries = [
        "Kazakhstan",
        "Uzbekistan",
        "Kyrgyzstan",
        "Tajikistan",
        "Turkmenistan",
        "Afghanistan",
        "Mongolia",
        "Xinjiang",
    ];
    let periods = [
        "Silk Road",
        "Mongol Empire",
        "Timurid",
        "Russian Imperial",
        "Soviet",
        "Post-Soviet",
    ];
    let lenses = [
        "History",
        "Political Economy",
        "Linguistics",
        "Anthropology",
        "Islam Studies",
        "Environmental",
        "Security",
    ];
    let language_families = ["Turkic", "Iranian", "Mongolic", "Russian"];
    let nomadic_options = ["Nomadic", "Settled", "Mixed", "Urban Contemporary"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Central Asian Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Country" }
                    select {
                        value: "{country}",
                        onchange: move |e| country.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in countries { option { value: "{x}", "{x}" } }
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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Language Family" }
                    select {
                        value: "{language_family}",
                        onchange: move |e| language_family.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in language_families { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Nomadic vs Settled" }
                    select {
                        value: "{nomadic_vs_settled}",
                        onchange: move |e| nomadic_vs_settled.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in nomadic_options { option { value: "{x}", "{x}" } }
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{country} | {period} | {disciplinary_lens} | {language_family} | {nomadic_vs_settled}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → silk road engine | steppe culture sieve | geospatial anchor" }
            }
        }
    }
}
