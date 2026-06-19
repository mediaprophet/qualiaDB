use dioxus::prelude::*;

#[component]
pub fn CreativeWritingQapp() -> Element {
    let mut genre = use_signal(|| "Literary Fiction".to_string());
    let mut arc_stage = use_signal(|| "Rising Action".to_string());
    let mut pov = use_signal(|| "Third Person Limited".to_string());
    let mut tone = use_signal(|| "Melancholic".to_string());
    let mut word_count = use_signal(|| 0usize);
    let mut text = use_signal(|| String::new());
    let mut protagonist = use_signal(|| String::new());
    let mut central_conflict = use_signal(|| String::new());

    let genres = [
        "Literary Fiction",
        "Speculative Fiction",
        "Historical Fiction",
        "Magical Realism",
        "Horror",
        "Romance",
        "Thriller",
        "Essay / Creative Non-Fiction",
        "Poetry",
        "Graphic Narrative",
    ];
    let stages = [
        "Exposition",
        "Rising Action",
        "Climax",
        "Falling Action",
        "Denouement",
    ];
    let povs = [
        "First Person",
        "Second Person",
        "Third Person Limited",
        "Third Person Omniscient",
        "Multiple POV",
    ];
    let tones = [
        "Melancholic",
        "Satirical",
        "Lyrical",
        "Terse / Minimalist",
        "Gothic",
        "Comic",
        "Elegiac",
        "Urgent",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Creative Writing Studio" }

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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Narrative Arc Stage" }
                    select {
                        value: "{arc_stage}",
                        onchange: move |e| arc_stage.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for s in stages { option { value: "{s}", "{s}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Point of View" }
                    select {
                        value: "{pov}",
                        onchange: move |e| pov.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for p in povs { option { value: "{p}", "{p}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tone / Voice" }
                    select {
                        value: "{tone}",
                        onchange: move |e| tone.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for t in tones { option { value: "{t}", "{t}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Protagonist" }
                    input {
                        r#type: "text", placeholder: "Name and brief description…",
                        value: "{protagonist}",
                        oninput: move |e| protagonist.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Central Conflict" }
                    input {
                        r#type: "text", placeholder: "Person vs. self, society, nature…",
                        value: "{central_conflict}",
                        oninput: move |e| central_conflict.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Draft Text — {word_count} words" }
                textarea {
                    value: "{text}",
                    oninput: move |e| {
                        let val = e.value();
                        word_count.set(val.split_whitespace().count());
                        text.set(val);
                    },
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; font-family: Georgia, serif; font-size: 0.9rem; line-height: 1.6; box-sizing: border-box; min-height: 120px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); display: flex; gap: 24px; flex-wrap: wrap;",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Genre: {genre}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Stage: {arc_stage}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "POV: {pov}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tone: {tone}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-accent); font-weight: bold;", "{word_count} words" }
            }
        }
    }
}
