use dioxus::prelude::*;

#[component]
pub fn ChildrensLiteratureQapp() -> Element {
    let mut age_group = use_signal(|| "Picture Book 3–6".to_string());
    let mut genre = use_signal(|| "Fantasy".to_string());
    let mut narrative_mode = use_signal(|| "Third Person".to_string());
    let mut reading_level = use_signal(|| 500u32);
    let mut theme = use_signal(|| "Coming-of-Age".to_string());
    let mut notes = use_signal(|| String::new());

    let age_groups = [
        "Infant/Toddler",
        "Picture Book 3–6",
        "Early Reader 6–9",
        "Middle Grade 9–12",
        "Young Adult 12–18",
    ];
    let genres = [
        "Fantasy",
        "Realistic Fiction",
        "Historical",
        "Adventure",
        "Mystery",
        "Informational",
        "Poetry",
    ];
    let narrative_modes = ["First Person", "Third Person", "Second Person"];
    let themes = [
        "Coming-of-Age",
        "Friendship",
        "Family",
        "Identity",
        "Adventure",
        "Social Justice",
        "Nature",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Children's Literature" }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Age Group" }
                select {
                    value: "{age_group}",
                    onchange: move |e| age_group.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in age_groups { option { value: "{x}", "{x}" } }
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
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Narrative Mode" }
                select {
                    value: "{narrative_mode}",
                    onchange: move |e| narrative_mode.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in narrative_modes { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Reading Level (Lexile): {reading_level}" }
                input {
                    r#type: "range", min: "0", max: "1500",
                    value: "{reading_level}",
                    oninput: move |e| reading_level.set(e.value().parse().unwrap_or(500)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theme" }
                select {
                    value: "{theme}",
                    onchange: move |e| theme.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in themes { option { value: "{x}", "{x}" } }
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{age_group} | {genre} | {narrative_mode} | Lexile: {reading_level} | {theme}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
