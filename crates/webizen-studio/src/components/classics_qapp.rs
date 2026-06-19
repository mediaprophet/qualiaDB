use dioxus::prelude::*;

#[component]
pub fn ClassicsQapp() -> Element {
    let mut language = use_signal(|| "Latin".to_string());
    let mut text_input = use_signal(|| String::new());
    let mut date_bce = use_signal(|| 44i32);
    let mut metre = use_signal(|| "Dactylic Hexameter".to_string());
    let mut manuscript = use_signal(|| String::new());
    let mut author = use_signal(|| String::new());

    let metres = [
        "Dactylic Hexameter",
        "Elegiac Couplet",
        "Iambic Trimeter",
        "Sapphic Strophe",
        "Alcaic Strophe",
        "Prose",
    ];
    let languages = [
        "Latin",
        "Ancient Greek",
        "Classical Chinese",
        "Sanskrit",
        "Old Persian",
        "Akkadian",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Classics" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Language" }
                    select {
                        value: "{language}",
                        onchange: move |e| language.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for l in languages {
                            option { value: "{l}", "{l}" }
                        }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Metre / Form" }
                    select {
                        value: "{metre}",
                        onchange: move |e| metre.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for m in metres {
                            option { value: "{m}", "{m}" }
                        }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Date (BCE)" }
                    input {
                        r#type: "number",
                        value: "{date_bce}",
                        oninput: move |e| date_bce.set(e.value().parse().unwrap_or(44)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Author / Attributed Author" }
                    input {
                        r#type: "text",
                        placeholder: "e.g. Cicero, Vergil, Homer…",
                        value: "{author}",
                        oninput: move |e| author.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Manuscript / Source" }
                    input {
                        r#type: "text",
                        placeholder: "e.g. Codex Vaticanus, P.Oxy. 2192…",
                        value: "{manuscript}",
                        oninput: move |e| manuscript.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Primary Text (paste original)" }
                textarea {
                    value: "{text_input}",
                    oninput: move |e| text_input.set(e.value()),
                    rows: "5",
                    placeholder: "Arma virumque cano, Troiae qui primus ab oris…",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: vertical; font-family: serif; box-sizing: border-box;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Metrical / Philological Analysis" }
                div { style: "font-size: 0.85rem; color: var(--qualia-text-muted);",
                    "{language} | {metre} | ca. {date_bce} BCE"
                }
                if !author().is_empty() {
                    div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); margin-top: 4px;", "Author: {author}" }
                }
                if !manuscript().is_empty() {
                    div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); margin-top: 4px;", "Source: {manuscript}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → textual_critic engine | neuro-symbolic concordance sieve" }
            }
        }
    }
}
