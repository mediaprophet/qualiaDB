use dioxus::prelude::*;

#[component]
pub fn ScienceFictionAndFantasyStudiesQapp() -> Element {
    let mut genre = use_signal(|| "Hard SF".to_string());
    let mut theoretical_approach = use_signal(|| "Cognition Estrangement (Suvin)".to_string());
    let mut medium = use_signal(|| "Novel".to_string());
    let mut tech_or_magic_system = use_signal(|| String::new());
    let mut utopian_dystopian_spectrum = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let genres = [
        "Hard SF",
        "Soft SF",
        "Cyberpunk",
        "Solarpunk",
        "Space Opera",
        "Dystopian",
        "Utopian",
        "High Fantasy",
        "Dark Fantasy",
        "Urban Fantasy",
        "Slipstream",
        "Afrofuturism",
        "Indigenous Futurism",
    ];
    let approaches = [
        "Cognition Estrangement (Suvin)",
        "Sense of Wonder",
        "Feminist SF",
        "Postcolonial",
        "Afrofuturist",
        "Ecocritical",
        "Posthumanist",
        "Marxist",
    ];
    let mediums = [
        "Novel",
        "Short Story",
        "Film",
        "TV",
        "Game",
        "Comics",
        "Podcast",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Science Fiction & Fantasy Studies" }

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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Medium" }
                    select {
                        value: "{medium}",
                        onchange: move |e| medium.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in mediums { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tech / Magic System" }
                    input {
                        r#type: "text",
                        value: "{tech_or_magic_system}",
                        oninput: move |e| tech_or_magic_system.set(e.value()),
                        placeholder: "e.g. FTL drives, hard magic system, AI consciousness",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Utopian↔Dystopian Spectrum: {utopian_dystopian_spectrum} (0=Dystopia, 100=Utopia)" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "100",
                    value: "{utopian_dystopian_spectrum}",
                    oninput: move |e| utopian_dystopian_spectrum.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{genre} | {medium} | {theoretical_approach} | Spectrum:{utopian_dystopian_spectrum}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → SFF corpus engine | novum sieve | estrangement anchor" }
            }
        }
    }
}
