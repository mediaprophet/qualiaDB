use dioxus::prelude::*;

#[component]
pub fn ComicsAndGraphicNovelStudiesQapp() -> Element {
    let mut genre = use_signal(|| "Superhero".to_string());
    let mut theoretical_approach = use_signal(|| "Narratology".to_string());
    let mut formal_element = use_signal(|| "Panel Composition".to_string());
    let mut tradition = use_signal(|| "American Silver Age".to_string());
    let mut creator_or_work = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    let genres = [
        "Superhero",
        "Slice-of-Life",
        "Horror",
        "Sci-Fi",
        "Political",
        "Memoir",
        "Literary",
        "Manga",
        "Manhwa",
        "Bande Dessinée",
        "Webcomic",
    ];
    let approaches = [
        "Narratology",
        "Semiotics",
        "Cultural Studies",
        "Feminist",
        "Postcolonial",
        "Industry Studies",
        "Reader Response",
    ];
    let formal_elements = [
        "Panel Composition",
        "Gutters",
        "Speech Bubbles",
        "Colour",
        "Line Weight",
        "Page Turn",
        "Sequence",
    ];
    let traditions = [
        "American Golden Age",
        "American Silver Age",
        "Bronze Age",
        "Indie",
        "Manga",
        "European BD",
        "Contemporary Global",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Comics & Graphic Novel Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Approach" }
                    select {
                        value: "{theoretical_approach}",
                        onchange: move |e| theoretical_approach.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in approaches { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Formal Element" }
                    select {
                        value: "{formal_element}",
                        onchange: move |e| formal_element.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in formal_elements { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tradition" }
                    select {
                        value: "{tradition}",
                        onchange: move |e| tradition.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in traditions { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Creator or Work" }
                input {
                    r#type: "text",
                    value: "{creator_or_work}",
                    oninput: move |e| creator_or_work.set(e.value()),
                    placeholder: "e.g. Art Spiegelman Maus, Alan Moore Watchmen, Osamu Tezuka",
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{genre} | {tradition} | {theoretical_approach} | {formal_element}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → comics studies engine | sequential art sieve | formal element anchor" }
            }
        }
    }
}
