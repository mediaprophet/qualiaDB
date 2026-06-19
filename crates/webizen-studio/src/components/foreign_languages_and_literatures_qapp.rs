use dioxus::prelude::*;

#[component]
pub fn ForeignLanguagesAndLiteraturesQapp() -> Element {
    let mut source_lang = use_signal(|| "French".to_string());
    let mut target_lang = use_signal(|| "English".to_string());
    let mut register = use_signal(|| "Literary / Formal".to_string());
    let mut source_text = use_signal(|| String::new());
    let mut cultural_gloss = use_signal(|| String::new());
    let mut period = use_signal(|| "20th Century".to_string());
    let mut translation_strategy = use_signal(|| "Foreignizing".to_string());

    let languages = [
        "Arabic",
        "Mandarin Chinese",
        "French",
        "German",
        "Italian",
        "Japanese",
        "Korean",
        "Portuguese",
        "Russian",
        "Spanish",
        "Swahili",
        "Turkish",
        "Hindi",
        "Persian (Farsi)",
        "Ancient Greek",
        "Latin",
        "Hebrew",
        "Dutch",
        "Swedish",
        "Polish",
    ];
    let registers = [
        "Literary / Formal",
        "Colloquial / Everyday",
        "Academic / Technical",
        "Sacred / Liturgical",
        "Journalistic",
        "Legal / Official",
    ];
    let periods = [
        "Ancient / Classical",
        "Medieval",
        "Early Modern (16th–17th C.)",
        "Enlightenment (18th C.)",
        "Romantic (19th C.)",
        "20th Century",
        "Contemporary",
    ];
    let strategies = [
        "Foreignizing (Venuti)",
        "Domesticating (Nida)",
        "Literal / Interlinear",
        "Dynamic Equivalence",
        "Communicative",
        "Semantic",
        "Interpretive",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Foreign Languages & Literatures" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Source Language" }
                    select {
                        value: "{source_lang}",
                        onchange: move |e| source_lang.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for l in languages { option { value: "{l}", "{l}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Target Language" }
                    select {
                        value: "{target_lang}",
                        onchange: move |e| target_lang.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for l in languages { option { value: "{l}", "{l}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Register" }
                    select {
                        value: "{register}",
                        onchange: move |e| register.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for r in registers { option { value: "{r}", "{r}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Literary Period" }
                    select {
                        value: "{period}",
                        onchange: move |e| period.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for p in periods { option { value: "{p}", "{p}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Source Text" }
                textarea {
                    value: "{source_text}",
                    oninput: move |e| source_text.set(e.value()),
                    rows: "4",
                    placeholder: "Paste original language text here…",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: vertical; font-family: serif; box-sizing: border-box;"
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Translation Strategy" }
                    select {
                        value: "{translation_strategy}",
                        onchange: move |e| translation_strategy.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for s in strategies { option { value: "{s}", "{s}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Cultural Gloss / Annotation" }
                    input {
                        r#type: "text", placeholder: "Untranslatable terms, cultural context notes…",
                        value: "{cultural_gloss}",
                        oninput: move |e| cultural_gloss.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Translation Analysis" }
                div { style: "font-size: 0.85rem; color: var(--qualia-text);", "{source_lang} → {target_lang}" }
                div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); margin-top: 4px;",
                    "Strategy: {translation_strategy} | Register: {register} | Period: {period}"
                }
                if !cultural_gloss().is_empty() {
                    div { style: "font-size: 0.8rem; color: var(--qualia-accent); margin-top: 4px;", "Cultural note: {cultural_gloss}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → translation studies engine | lexical graph | neuro-symbolic sieve" }
            }
        }
    }
}
