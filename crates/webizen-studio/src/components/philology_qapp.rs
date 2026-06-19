use dioxus::prelude::*;

#[component]
pub fn PhilologyQapp() -> Element {
    let mut language_family = use_signal(|| "Indo-European".to_string());
    let mut philological_method = use_signal(|| "Comparative Method".to_string());
    let mut manuscript_tradition = use_signal(|| String::new());
    let mut attested_date_ce = use_signal(|| String::new());
    let mut text_genre = use_signal(|| "Literary Prose".to_string());
    let mut critical_apparatus = use_signal(|| String::new());

    let families = [
        "Indo-European",
        "Semitic",
        "Sino-Tibetan",
        "Dravidian",
        "Turkic",
        "Afro-Asiatic",
        "Austronesian",
        "Uralic",
    ];
    let methods = [
        "Textual Criticism",
        "Stemmatic Analysis",
        "Comparative Method",
        "Internal Reconstruction",
        "Lachmann Method",
        "Computer-Assisted Stemma",
    ];
    let genres = [
        "Epic",
        "Religious Text",
        "Legal",
        "Administrative",
        "Literary Prose",
        "Poetry",
        "Letter",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Philology" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Language Family" }
                    select {
                        value: "{language_family}",
                        onchange: move |e| language_family.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in families { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Philological Method" }
                    select {
                        value: "{philological_method}",
                        onchange: move |e| philological_method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in methods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Text Genre" }
                    select {
                        value: "{text_genre}",
                        onchange: move |e| text_genre.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in genres { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Attested Date CE" }
                    input {
                        r#type: "text",
                        value: "{attested_date_ce}",
                        oninput: move |e| attested_date_ce.set(e.value()),
                        placeholder: "e.g. 800 CE, 13th c.",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Manuscript Tradition" }
                input {
                    r#type: "text",
                    value: "{manuscript_tradition}",
                    oninput: move |e| manuscript_tradition.set(e.value()),
                    placeholder: "e.g. Codex Sinaiticus, Dead Sea Scrolls, Beowulf MS",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Critical Apparatus Notes" }
                textarea {
                    value: "{critical_apparatus}",
                    oninput: move |e| critical_apparatus.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 80px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{language_family} | {philological_method} | {text_genre} | {attested_date_ce}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → stemmatic engine | manuscript corpus | comparative sieve" }
            }
        }
    }
}
