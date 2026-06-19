use dioxus::prelude::*;

#[component]
pub fn ComparativeLiteratureQapp() -> Element {
    let mut primary_text = use_signal(|| String::new());
    let mut secondary_text = use_signal(|| String::new());
    let mut primary_culture = use_signal(|| String::new());
    let mut secondary_culture = use_signal(|| String::new());
    let mut thematic_axis = use_signal(|| "Hero's Journey".to_string());
    let mut influence_direction = use_signal(|| "Bidirectional".to_string());
    let mut translation_mode = use_signal(|| false);

    let themes = [
        "Hero's Journey",
        "Tragic Fall",
        "Love and Loss",
        "Exile and Return",
        "Sacred vs. Profane",
        "Nature and Civilization",
        "Identity and Otherness",
        "Power and Resistance",
    ];
    let directions = ["Bidirectional", "A → B", "B → A", "Independent Parallel"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Comparative Literature" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Thematic Axis" }
                    select {
                        value: "{thematic_axis}",
                        onchange: move |e| thematic_axis.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for t in themes {
                            option { value: "{t}", "{t}" }
                        }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Influence Direction" }
                    select {
                        value: "{influence_direction}",
                        onchange: move |e| influence_direction.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for d in directions {
                            option { value: "{d}", "{d}" }
                        }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Text A — Cultural Origin" }
                    input {
                        r#type: "text",
                        placeholder: "e.g. Ancient Greek, 19th-century Russian…",
                        value: "{primary_culture}",
                        oninput: move |e| primary_culture.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                    textarea {
                        value: "{primary_text}",
                        oninput: move |e| primary_text.set(e.value()),
                        rows: "4",
                        placeholder: "Paste or describe Text A…",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: vertical; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Text B — Cultural Origin" }
                    input {
                        r#type: "text",
                        placeholder: "e.g. Meiji-era Japanese, Colombian magical realism…",
                        value: "{secondary_culture}",
                        oninput: move |e| secondary_culture.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                    textarea {
                        value: "{secondary_text}",
                        oninput: move |e| secondary_text.set(e.value()),
                        rows: "4",
                        placeholder: "Paste or describe Text B…",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: vertical; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "display: flex; align-items: center; gap: 8px;",
                input {
                    r#type: "checkbox",
                    checked: "{translation_mode}",
                    onchange: move |e| translation_mode.set(e.checked()),
                    id: "trans-mode"
                }
                label { r#for: "trans-mode", style: "font-size: 0.85rem; color: var(--qualia-text-muted); cursor: pointer;", "Translation Comparison Mode" }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Thematic Resonance Output" }
                div { style: "font-size: 0.85rem; color: var(--qualia-text-muted);",
                    "Theme: {thematic_axis} | Influence: {influence_direction}"
                }
                div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); margin-top: 4px;",
                    "{primary_culture} ↔ {secondary_culture}"
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → neuro-symbolic sieve | graph similarity engine" }
            }
        }
    }
}
