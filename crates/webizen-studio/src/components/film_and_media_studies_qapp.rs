use dioxus::prelude::*;

#[component]
pub fn FilmAndMediaStudiesQapp() -> Element {
    let mut genre = use_signal(|| "Drama".to_string());
    let mut narrative_structure = use_signal(|| "Three-Act".to_string());
    let mut shot_type = use_signal(|| "Medium Shot".to_string());
    let mut camera_movement = use_signal(|| "Static".to_string());
    let mut auteur = use_signal(|| String::new());
    let mut scene_notes = use_signal(|| String::new());
    let mut aspect_ratio = use_signal(|| "1.85:1".to_string());

    let genres = [
        "Drama",
        "Comedy",
        "Thriller",
        "Horror",
        "Sci-Fi",
        "Documentary",
        "Experimental / Avant-Garde",
        "Animation",
        "Musical",
        "Western",
        "Film Noir",
    ];
    let structures = [
        "Three-Act",
        "Hero's Journey",
        "In Medias Res",
        "Episodic",
        "Non-Linear / Fragmented",
        "Kishōtenketsu",
        "Five-Act (Classical)",
    ];
    let shots = [
        "Extreme Wide Shot",
        "Wide Shot",
        "Medium Wide",
        "Medium Shot",
        "Medium Close-Up",
        "Close-Up",
        "Extreme Close-Up",
        "Over-the-Shoulder",
        "Two Shot",
        "Insert",
    ];
    let movements = [
        "Static",
        "Pan Left",
        "Pan Right",
        "Tilt Up",
        "Tilt Down",
        "Dolly In",
        "Dolly Out",
        "Crane / Jib",
        "Steadicam",
        "Handheld",
        "Zoom",
    ];
    let ratios = [
        "1.33:1 (4:3)",
        "1.78:1 (16:9)",
        "1.85:1",
        "2.39:1 (Anamorphic)",
        "2.20:1 (70mm)",
        "1.43:1 (IMAX)",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Film & Media Studies" }

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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Narrative Structure" }
                    select {
                        value: "{narrative_structure}",
                        onchange: move |e| narrative_structure.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for s in structures { option { value: "{s}", "{s}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Shot Type" }
                    select {
                        value: "{shot_type}",
                        onchange: move |e| shot_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for s in shots { option { value: "{s}", "{s}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Camera Movement" }
                    select {
                        value: "{camera_movement}",
                        onchange: move |e| camera_movement.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for m in movements { option { value: "{m}", "{m}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Director / Auteur" }
                    input {
                        r#type: "text", placeholder: "e.g. Agnès Varda, Akira Kurosawa…",
                        value: "{auteur}",
                        oninput: move |e| auteur.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Aspect Ratio" }
                    select {
                        value: "{aspect_ratio}",
                        onchange: move |e| aspect_ratio.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for r in ratios { option { value: "{r}", "{r}" } }
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Scene / Sequence Analysis Notes" }
                textarea {
                    value: "{scene_notes}",
                    oninput: move |e| scene_notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 80px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); display: flex; gap: 16px; flex-wrap: wrap;",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Genre: {genre}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Structure: {narrative_structure}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Shot: {shot_type}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Move: {camera_movement}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "AR: {aspect_ratio}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); width: 100%;", "QualiaDB → narrative_structure engine | visual semiotics sieve" }
            }
        }
    }
}
