use dioxus::prelude::*;

#[component]
pub fn TranslationStudiesQapp() -> Element {
    let mut theoretical_approach = use_signal(|| "Descriptive Translation Studies".to_string());
    let mut source_language = use_signal(|| "English".to_string());
    let mut target_language = use_signal(|| "French".to_string());
    let mut text_type = use_signal(|| "Literary".to_string());
    let mut domestication_foreignization = use_signal(|| 3u32);
    let mut cultural_untranslatability = use_signal(|| String::new());
    let mut translation_excerpt = use_signal(|| String::new());

    let approaches = [
        "Equivalence Theory",
        "Skopos",
        "Polysystem Theory",
        "Descriptive Translation Studies",
        "Post-Colonial Translation",
        "Feminist Translation",
        "Machine Translation Ethics",
        "Localisation",
    ];
    let text_types = [
        "Literary",
        "Legal",
        "Technical",
        "Religious",
        "Medical",
        "Audiovisual",
        "Sign Language",
    ];
    let df_labels = [
        "Strongly Domesticating",
        "Domesticating",
        "Neutral",
        "Foreignizing",
        "Strongly Foreignizing",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Translation Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Approach" }
                    select {
                        value: "{theoretical_approach}",
                        onchange: move |e| theoretical_approach.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in approaches { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Text Type" }
                    select {
                        value: "{text_type}",
                        onchange: move |e| text_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in text_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Source Language" }
                    input {
                        r#type: "text",
                        value: "{source_language}",
                        oninput: move |e| source_language.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Target Language" }
                    input {
                        r#type: "text",
                        value: "{target_language}",
                        oninput: move |e| target_language.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);",
                    "Domestication / Foreignization (1=Strongly Domesticating … 5=Strongly Foreignizing): {domestication_foreignization}"
                }
                input {
                    r#type: "range",
                    min: "1",
                    max: "5",
                    value: "{domestication_foreignization}",
                    oninput: move |e| domestication_foreignization.set(e.value().parse().unwrap_or(3)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Cultural Untranslatability" }
                input {
                    r#type: "text",
                    value: "{cultural_untranslatability}",
                    oninput: move |e| cultural_untranslatability.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Translation Excerpt" }
                textarea {
                    value: "{translation_excerpt}",
                    oninput: move |e| translation_excerpt.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 80px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{theoretical_approach} | {source_language} → {target_language} | {text_type}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → equivalence engine | polysystem sieve | cultural transfer graph" }
            }
        }
    }
}
