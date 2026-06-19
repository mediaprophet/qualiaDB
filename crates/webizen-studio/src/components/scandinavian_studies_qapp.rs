use dioxus::prelude::*;

#[component]
pub fn ScandinavianStudiesQapp() -> Element {
    let mut country = use_signal(|| "Sweden".to_string());
    let mut period = use_signal(|| "Welfare State".to_string());
    let mut subfield = use_signal(|| "Political Economy".to_string());
    let mut language = use_signal(|| "Swedish".to_string());
    let mut nordic_model_dimension = use_signal(|| "Universal Welfare".to_string());
    let mut notes = use_signal(|| String::new());

    let countries = [
        "Sweden",
        "Norway",
        "Denmark",
        "Finland",
        "Iceland",
        "Faroe Islands",
        "Greenland",
        "Nordic Region",
    ];
    let periods = [
        "Viking Age",
        "Medieval",
        "Early Modern",
        "Industrialisation",
        "Welfare State",
        "Contemporary",
    ];
    let subfields = [
        "Literature",
        "History",
        "Political Economy",
        "Folklore",
        "Linguistics",
        "Film",
        "Design",
        "Nordic Model Analysis",
    ];
    let languages = [
        "Old Norse",
        "Swedish",
        "Norwegian Bokmål",
        "Norwegian Nynorsk",
        "Danish",
        "Finnish",
        "Icelandic",
    ];
    let dimensions = [
        "Universal Welfare",
        "Labour Relations",
        "Gender Equality",
        "Environmental Policy",
        "Digital Governance",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Scandinavian Studies" }

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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Subfield" }
                    select {
                        value: "{subfield}",
                        onchange: move |e| subfield.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in subfields { option { value: "{x}", "{x}" } }
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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Nordic Model Dimension" }
                    select {
                        value: "{nordic_model_dimension}",
                        onchange: move |e| nordic_model_dimension.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in dimensions { option { value: "{x}", "{x}" } }
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{country} | {period} | {subfield} | {nordic_model_dimension}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → Nordic model engine | comparative welfare sieve | social democratic graph" }
            }
        }
    }
}
