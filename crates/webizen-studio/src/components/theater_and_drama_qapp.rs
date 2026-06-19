use dioxus::prelude::*;

#[component]
pub fn TheaterAndDramaQapp() -> Element {
    let mut genre = use_signal(|| "Tragedy".to_string());
    let mut dramatic_structure = use_signal(|| "Five-Act (Aristotelian)".to_string());
    let mut convention = use_signal(|| "Naturalism / Realism".to_string());
    let mut playwright = use_signal(|| String::new());
    let mut character_name = use_signal(|| String::new());
    let mut character_objective = use_signal(|| String::new());
    let mut scene_notes = use_signal(|| String::new());
    let mut staging_config = use_signal(|| "Proscenium".to_string());

    let genres = [
        "Tragedy",
        "Comedy",
        "Tragicomedy",
        "Farce",
        "Melodrama",
        "Epic Theatre (Brecht)",
        "Theatre of the Absurd",
        "Musical Theatre",
        "Documentary Theatre / Verbatim",
        "Immersive / Environmental",
    ];
    let structures = [
        "Five-Act (Aristotelian)",
        "Three-Act",
        "Brechtian Episodes",
        "In Medias Res",
        "Two-Hander",
        "Monodrama",
        "Greek Chorus Structure",
    ];
    let conventions = [
        "Naturalism / Realism",
        "Expressionism",
        "Surrealism",
        "Symbolism",
        "Commedia dell'arte",
        "Noh",
        "Kabuki",
        "Physical Theatre",
        "Postdramatic",
    ];
    let stagings = [
        "Proscenium",
        "Thrust",
        "In-the-Round (Arena)",
        "Traverse",
        "Black Box",
        "Site-Specific",
        "Promenade",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Theater & Drama" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Genre" }
                    select {
                        value: "{genre}",
                        onchange: move |e| genre.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for g in genres { option { value: "{g}", "{g}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Dramatic Structure" }
                    select {
                        value: "{dramatic_structure}",
                        onchange: move |e| dramatic_structure.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for s in structures { option { value: "{s}", "{s}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theatrical Convention" }
                    select {
                        value: "{convention}",
                        onchange: move |e| convention.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for c in conventions { option { value: "{c}", "{c}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Staging Configuration" }
                    select {
                        value: "{staging_config}",
                        onchange: move |e| staging_config.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for s in stagings { option { value: "{s}", "{s}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Playwright / Dramatist" }
                input {
                    r#type: "text", placeholder: "e.g. Sophocles, Ibsen, Caryl Churchill, Suzan-Lori Parks…",
                    value: "{playwright}",
                    oninput: move |e| playwright.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Character Name" }
                    input {
                        r#type: "text", placeholder: "Character to analyse…",
                        value: "{character_name}",
                        oninput: move |e| character_name.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Stanislavski Objective" }
                    input {
                        r#type: "text", placeholder: "What does the character want? (use an active verb)",
                        value: "{character_objective}",
                        oninput: move |e| character_objective.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Scene / Dramaturgy Notes" }
                textarea {
                    value: "{scene_notes}",
                    oninput: move |e| scene_notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); display: flex; gap: 16px; flex-wrap: wrap;",
                span { style: "font-size: 0.8rem; color: var(--qualia-text); font-weight: bold;", "{genre}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{dramatic_structure}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{convention}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{staging_config}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); width: 100%;", "QualiaDB → narrative_structure engine | character ontology | Allen Interval act-timing" }
            }
        }
    }
}
