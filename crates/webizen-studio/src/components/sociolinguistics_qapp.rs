use dioxus::prelude::*;

#[component]
pub fn SociolinguisticsQapp() -> Element {
    let mut phenomenon = use_signal(|| "Code-Switching".to_string());
    let mut sociolinguistic_variable = use_signal(|| String::new());
    let mut speech_community = use_signal(|| String::new());
    let mut method = use_signal(|| "Labovian Quantitative".to_string());
    let mut style_shifting_context = use_signal(|| "Casual".to_string());
    let mut prestige_type = use_signal(|| "Overt".to_string());
    let mut notes = use_signal(|| String::new());

    let phenomena = [
        "Code-Switching",
        "Diglossia",
        "Language Change",
        "Language Death",
        "Pidgin",
        "Creole",
        "Register Variation",
        "Accent",
        "Dialect",
        "Language Ideology",
        "Multilingualism",
    ];
    let methods = [
        "Labovian Quantitative",
        "Ethnographic",
        "Discourse Analysis",
        "Matched Guise",
        "Social Network Analysis",
        "Corpus",
    ];
    let style_contexts = ["Formal", "Casual", "Online", "In-Group", "Out-Group"];
    let prestige_types = ["Overt", "Covert"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Sociolinguistics" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Phenomenon" }
                    select {
                        value: "{phenomenon}",
                        onchange: move |e| phenomenon.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in phenomena { option { value: "{x}", "{x}" } }
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
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Style-Shifting Context" }
                    select {
                        value: "{style_shifting_context}",
                        onchange: move |e| style_shifting_context.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in style_contexts { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Prestige Type" }
                    select {
                        value: "{prestige_type}",
                        onchange: move |e| prestige_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in prestige_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sociolinguistic Variable" }
                    input {
                        r#type: "text",
                        value: "{sociolinguistic_variable}",
                        oninput: move |e| sociolinguistic_variable.set(e.value()),
                        placeholder: "e.g. /r/-dropping, (ING), H-dropping",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Speech Community" }
                    input {
                        r#type: "text",
                        value: "{speech_community}",
                        oninput: move |e| speech_community.set(e.value()),
                        placeholder: "e.g. New York City, Edinburgh, Lagos",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{phenomenon} | {speech_community} | {method} | {style_shifting_context} | {prestige_type} prestige" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → sociolinguistic corpus | variation sieve | community anchor" }
            }
        }
    }
}
