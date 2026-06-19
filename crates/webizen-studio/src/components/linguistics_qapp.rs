use dioxus::prelude::*;

#[component]
pub fn LinguisticsQapp() -> Element {
    let mut analysis_level = use_signal(|| "Syntax".to_string());
    let mut language_family = use_signal(|| "Indo-European".to_string());
    let mut ipa_input = use_signal(|| String::new());
    let mut morpheme_input = use_signal(|| String::new());
    let mut tree_notation = use_signal(|| String::new());
    let mut pragmatic_context = use_signal(|| String::new());

    let levels = [
        "Phonology",
        "Morphology",
        "Syntax",
        "Semantics",
        "Pragmatics",
        "Discourse Analysis",
        "Sociolinguistics",
        "Psycholinguistics",
    ];
    let families = [
        "Indo-European",
        "Sino-Tibetan",
        "Afro-Asiatic",
        "Niger-Congo",
        "Austronesian",
        "Turkic",
        "Dravidian",
        "Japonic",
        "Koreanic",
        "Uralic",
        "Isolate",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Linguistics" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Analysis Level" }
                    select {
                        value: "{analysis_level}",
                        onchange: move |e| analysis_level.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for l in levels { option { value: "{l}", "{l}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Language Family" }
                    select {
                        value: "{language_family}",
                        onchange: move |e| language_family.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for f in families { option { value: "{f}", "{f}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "IPA Transcription" }
                input {
                    r#type: "text",
                    placeholder: "/ˈlɪŋ.ɡwɪ.stɪks/ — paste or type IPA symbols…",
                    value: "{ipa_input}",
                    oninput: move |e| ipa_input.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; font-family: serif; box-sizing: border-box;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Morpheme Breakdown (use | to separate)" }
                input {
                    r#type: "text",
                    placeholder: "un|break|able | re|construct|ion|s",
                    value: "{morpheme_input}",
                    oninput: move |e| morpheme_input.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Phrase Structure / Dependency Tree (bracket notation)" }
                textarea {
                    value: "{tree_notation}",
                    oninput: move |e| tree_notation.set(e.value()),
                    rows: "3",
                    placeholder: "[S [NP [Det The][N dog]] [VP [V chased][NP [Det the][N cat]]]]",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: vertical; font-family: monospace; box-sizing: border-box;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Pragmatic / Discourse Context" }
                textarea {
                    value: "{pragmatic_context}",
                    oninput: move |e| pragmatic_context.set(e.value()),
                    rows: "2",
                    placeholder: "Speech act, implicature, register, turn-taking observations…",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: vertical; box-sizing: border-box;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Analysis Output" }
                div { style: "font-size: 0.85rem; color: var(--qualia-text-muted);",
                    "Level: {analysis_level} | Family: {language_family}"
                }
                if !morpheme_input().is_empty() {
                    div { style: "margin-top: 8px; font-size: 0.85rem;",
                        "Morphemes: "
                        for morph in morpheme_input().split('|') {
                            span {
                                style: "background: var(--qualia-border); padding: 2px 6px; border-radius: 3px; margin-right: 4px; font-size: 0.8rem;",
                                "{morph.trim()}"
                            }
                        }
                    }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → syntax_modeler | morphological parser | neuro-symbolic sieve" }
            }
        }
    }
}
