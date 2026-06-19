use dioxus::prelude::*;

#[component]
pub fn OceanicAndPacificIslandStudiesQapp() -> Element {
    let mut region = use_signal(|| "Polynesia".to_string());
    let mut disciplinary_lens = use_signal(|| "History".to_string());
    let mut colonial_power = use_signal(|| "British".to_string());
    let mut period = use_signal(|| "Contemporary".to_string());
    let mut language_family = use_signal(|| "Austronesian".to_string());
    let mut notes = use_signal(|| String::new());

    let regions = [
        "Polynesia",
        "Melanesia",
        "Micronesia",
        "Australia",
        "Aotearoa NZ",
        "Hawaiian Islands",
        "Guam",
        "Fiji",
        "PNG",
        "Oceania-Wide",
    ];
    let lenses = [
        "History",
        "Anthropology",
        "Literature",
        "Environmental",
        "Political Self-Determination",
        "Language Revitalisation",
        "Indigenous Knowledge",
    ];
    let powers = [
        "British",
        "French",
        "American",
        "German",
        "Japanese",
        "Spanish",
        "Independent",
    ];
    let periods = ["Pre-Contact", "Colonial", "Decolonisation", "Contemporary"];
    let lang_families = ["Austronesian", "Papuan", "Aboriginal Australian"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Oceanic & Pacific Island Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Region" }
                    select {
                        value: "{region}",
                        onchange: move |e| region.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in regions { option { value: "{x}", "{x}" } }
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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Colonial Power" }
                    select {
                        value: "{colonial_power}",
                        onchange: move |e| colonial_power.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in powers { option { value: "{x}", "{x}" } }
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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Language Family" }
                    select {
                        value: "{language_family}",
                        onchange: move |e| language_family.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in lang_families { option { value: "{x}", "{x}" } }
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{region} | {disciplinary_lens} | {colonial_power} | {period}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → indigenous knowledge engine | decolonisation graph | language revitalisation sieve" }
            }
        }
    }
}
