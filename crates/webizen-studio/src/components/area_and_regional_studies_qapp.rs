use dioxus::prelude::*;

#[component]
pub fn AreaAndRegionalStudiesQapp() -> Element {
    let mut region = use_signal(|| "East Asia".to_string());
    let mut country_focus = use_signal(|| String::new());
    let mut disciplinary_lens = use_signal(|| "Political Economy".to_string());
    let mut language_competency = use_signal(|| "Reading".to_string());
    let mut time_period = use_signal(|| "Contemporary".to_string());
    let mut research_question = use_signal(|| String::new());

    let regions = [
        "East Asia",
        "Southeast Asia",
        "South Asia",
        "Central Asia",
        "Middle East & North Africa",
        "Sub-Saharan Africa",
        "Latin America & Caribbean",
        "Eastern Europe & Russia",
        "Western Europe",
        "North America",
        "Oceania & Pacific",
        "Arctic & Circumpolar",
        "Transnational / Diasporic",
    ];
    let lenses = [
        "Political Economy",
        "Cultural Studies",
        "Historical",
        "Anthropological",
        "Linguistic",
        "Security Studies",
        "Development Studies",
        "Religious Studies",
        "Environmental",
        "Gender & Sexuality",
        "Postcolonial",
    ];
    let lang_levels = [
        "None",
        "Reading",
        "Reading + Speaking",
        "Full Professional",
        "Native / Heritage",
    ];
    let periods = [
        "Pre-Colonial",
        "Colonial (19th–early 20th C.)",
        "Decolonisation (1940s–1970s)",
        "Cold War",
        "Post-Cold War (1990s)",
        "Contemporary (2000–)",
        "Long Durée / Deep History",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Area & Regional Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "World Region" }
                    select {
                        value: "{region}",
                        onchange: move |e| region.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in regions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Disciplinary Lens" }
                    select {
                        value: "{disciplinary_lens}",
                        onchange: move |e| disciplinary_lens.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in lenses { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Time Period" }
                    select {
                        value: "{time_period}",
                        onchange: move |e| time_period.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in periods { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Country / Sub-Region Focus" }
                    input {
                        r#type: "text", placeholder: "e.g. Vietnam, Oaxaca, the Sahel…",
                        value: "{country_focus}",
                        oninput: move |e| country_focus.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Language Competency (Regional Lang.)" }
                    select {
                        value: "{language_competency}",
                        onchange: move |e| language_competency.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in lang_levels { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Research Question / Fieldwork Notes" }
                textarea {
                    value: "{research_question}",
                    oninput: move |e| research_question.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 100px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); display: flex; gap: 16px; flex-wrap: wrap;",
                span { style: "font-size: 0.8rem; color: var(--qualia-accent); font-weight: bold;", "{region}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{disciplinary_lens}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{time_period}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); width: 100%;", "QualiaDB → knowledge graph | Allen Interval | geospatial sieve" }
            }
        }
    }
}
