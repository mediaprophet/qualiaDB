use dioxus::prelude::*;

#[component]
pub fn RomanceLanguagesAndLiteraturesQapp() -> Element {
    let mut language = use_signal(|| "French".to_string());
    let mut period = use_signal(|| "Medieval".to_string());
    let mut genre = use_signal(|| "Novel".to_string());
    let mut literary_movement = use_signal(|| "Trobar".to_string());
    let mut text_excerpt = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    let languages = [
        "French",
        "Spanish",
        "Portuguese",
        "Italian",
        "Romanian",
        "Catalan",
        "Occitan",
        "Galician",
        "Sardinian",
    ];
    let periods = [
        "Vulgar Latin",
        "Medieval",
        "Renaissance",
        "Classical",
        "Romantic",
        "Realist",
        "Modernist",
        "Contemporary",
    ];
    let genres = [
        "Chanson de Geste",
        "Troubadour Lyric",
        "Novel",
        "Drama",
        "Essay",
        "Poetry",
    ];
    let movements = [
        "Trobar",
        "Dolce Stil Novo",
        "Petrarchism",
        "Picaresque",
        "Romanticism",
        "Realism",
        "Surrealism",
        "Magical Realism",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Romance Languages & Literatures" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Period" }
                    select {
                        value: "{period}",
                        onchange: move |e| period.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in periods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Genre" }
                    select {
                        value: "{genre}",
                        onchange: move |e| genre.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in genres { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Literary Movement" }
                    select {
                        value: "{literary_movement}",
                        onchange: move |e| literary_movement.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in movements { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Text Excerpt" }
                textarea {
                    value: "{text_excerpt}",
                    oninput: move |e| text_excerpt.set(e.value()),
                    placeholder: "Paste a relevant text excerpt here...",
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    value: "{notes}",
                    oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{language} | {period} | {genre} | {literary_movement}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → romance corpus | literary movement sieve | philological engine" }
            }
        }
    }
}
