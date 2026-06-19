use dioxus::prelude::*;

#[component]
pub fn FilmPhilosophyQapp() -> Element {
    let mut philosophical_tradition = use_signal(|| "Phenomenology".to_string());
    let mut philosopher_filmmaker = use_signal(|| "Deleuze/Tarkovsky".to_string());
    let mut film_concept = use_signal(|| "Time-Image".to_string());
    let mut medium_specificity = use_signal(|| 50u32);
    let mut philosophical_depth = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let philosophical_traditions = [
        "Phenomenology",
        "Analytic",
        "Continental",
        "Cognitivism",
        "Neurophilosophy",
        "Ethics",
    ];
    let philosopher_filmmakers = [
        "Deleuze/Tarkovsky",
        "Cavell/Hollywood",
        "Bazin/Neorealism",
        "Merleau-Ponty/Sensory Cinema",
        "Rancière/Essay Film",
    ];
    let film_concepts = [
        "Time-Image",
        "Movement-Image",
        "Indexicality",
        "Cinematic Gaze",
        "Apparatus",
        "Montage",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Film Philosophy" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Philosophical Tradition" }
                select {
                    value: "{philosophical_tradition}", onchange: move |e| philosophical_tradition.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in philosophical_traditions { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Philosopher-Filmmaker Pair" }
                select {
                    value: "{philosopher_filmmaker}", onchange: move |e| philosopher_filmmaker.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in philosopher_filmmakers { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Film Concept" }
                select {
                    value: "{film_concept}", onchange: move |e| film_concept.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in film_concepts { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Medium Specificity: {medium_specificity}" }
                input { r#type: "range", min: "0", max: "100", value: "{medium_specificity}",
                    oninput: move |e| medium_specificity.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;" }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Philosophical Depth: {philosophical_depth}" }
                input { r#type: "range", min: "0", max: "100", value: "{philosophical_depth}",
                    oninput: move |e| philosophical_depth.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;" }
            }
            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea { value: "{notes}", oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;" }
            }
            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{philosophical_tradition} | {film_concept} | specificity: {medium_specificity} | depth: {philosophical_depth}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → film philosophy engine | discourse sieve | anchor" }
            }
        }
    }
}
