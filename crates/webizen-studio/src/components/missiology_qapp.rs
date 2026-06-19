use dioxus::prelude::*;

#[component]
pub fn MissiologyQapp() -> Element {
    let mut tradition = use_signal(|| "Protestant Evangelical".to_string());
    let mut missiological_model = use_signal(|| "Missio Dei".to_string());
    let mut geographic_focus = use_signal(|| String::new());
    let mut historical_period = use_signal(|| "Contemporary".to_string());
    let mut method = use_signal(|| "Church Planting".to_string());
    let mut notes = use_signal(|| String::new());

    let traditions = [
        "Catholic",
        "Protestant Evangelical",
        "Ecumenical",
        "Pentecostal",
        "Orthodox",
        "Postcolonial Mission Theory",
    ];
    let models = [
        "Christendom",
        "Three-Self Formula",
        "Contextualisation",
        "Kingdom Ethics",
        "Missio Dei",
        "Integral Mission",
        "Decolonial Mission",
    ];
    let periods = [
        "Early Church",
        "Medieval",
        "Colonial Missions",
        "20th C. Ecumenism",
        "Contemporary",
    ];
    let methods = [
        "Church Planting",
        "Medical",
        "Educational",
        "Development",
        "Dialogue",
        "Witness",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Missiology" }

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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Missiological Model" }
                    select {
                        value: "{missiological_model}",
                        onchange: move |e| missiological_model.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in models { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Historical Period" }
                    select {
                        value: "{historical_period}",
                        onchange: move |e| historical_period.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in periods { option { value: "{x}", "{x}" } }
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
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Geographic Focus" }
                input {
                    r#type: "text",
                    value: "{geographic_focus}",
                    oninput: move |e| geographic_focus.set(e.value()),
                    placeholder: "e.g. Sub-Saharan Africa, Southeast Asia, Urban Europe",
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{tradition} | {missiological_model} | {historical_period} | {method} | {geographic_focus}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → missiology engine | contextualisation sieve | cross-cultural anchor" }
            }
        }
    }
}
