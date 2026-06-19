use dioxus::prelude::*;

#[component]
pub fn MorphologyQapp() -> Element {
    let mut morphological_type = use_signal(|| "Agglutinative".to_string());
    let mut morphological_process = use_signal(|| "Affixation".to_string());
    let mut morpheme_type = use_signal(|| "Root".to_string());
    let mut language_example = use_signal(|| String::new());
    let mut word_form = use_signal(|| String::new());
    let mut paradigm_notes = use_signal(|| String::new());

    let morph_types = [
        "Isolating",
        "Agglutinative",
        "Fusional",
        "Polysynthetic",
        "Incorporation",
    ];
    let processes = [
        "Affixation",
        "Compounding",
        "Reduplication",
        "Conversion",
        "Clipping",
        "Blending",
        "Back-Formation",
        "Suppletion",
    ];
    let morpheme_types = [
        "Root",
        "Derivational Prefix",
        "Derivational Suffix",
        "Inflectional",
        "Clitic",
        "Zero Morpheme",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Morphology" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Morphological Type" }
                    select {
                        value: "{morphological_type}",
                        onchange: move |e| morphological_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in morph_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Morphological Process" }
                    select {
                        value: "{morphological_process}",
                        onchange: move |e| morphological_process.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in processes { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Morpheme Type" }
                    select {
                        value: "{morpheme_type}",
                        onchange: move |e| morpheme_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in morpheme_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Language Example" }
                    input {
                        r#type: "text",
                        value: "{language_example}",
                        oninput: move |e| language_example.set(e.value()),
                        placeholder: "e.g. Turkish, Swahili, Latin, Mandarin",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Word Form / Morpheme Analysis" }
                textarea {
                    value: "{word_form}",
                    oninput: move |e| word_form.set(e.value()),
                    placeholder: "e.g. un-break-able = un- (neg.) + break (root) + -able (adj. suffix)",
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Paradigm Notes" }
                textarea {
                    value: "{paradigm_notes}",
                    oninput: move |e| paradigm_notes.set(e.value()),
                    placeholder: "Describe the inflectional paradigm or morphological paradigm here...",
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{morphological_type} | {morphological_process} | {morpheme_type} | {language_example}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → morphological analysis engine | paradigm sieve | word-form anchor" }
            }
        }
    }
}
