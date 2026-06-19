use dioxus::prelude::*;

#[component]
pub fn PoetryAndPoeticsQapp() -> Element {
    let mut tradition = use_signal(|| "Lyric".to_string());
    let mut theoretical_approach = use_signal(|| "New Critical".to_string());
    let mut prosody = use_signal(|| "Iambic".to_string());
    let mut rhetorical_figure = use_signal(|| "Metaphor".to_string());
    let mut poem_excerpt = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    let traditions = [
        "Lyric",
        "Epic",
        "Dramatic",
        "Confessional",
        "Language Poetry",
        "Experimental",
        "Spoken Word",
        "Slam",
        "Indigenous",
        "Ghazal",
        "Haiku",
        "Sonnet",
        "Villanelle",
        "Free Verse",
    ];
    let approaches = [
        "Formalist",
        "New Critical",
        "Cognitive Poetics",
        "Feminist",
        "Postcolonial",
        "Ecocritical",
        "Phenomenological",
        "Computational",
    ];
    let prosodies = [
        "Iambic",
        "Trochaic",
        "Dactylic",
        "Anapestic",
        "Free Verse",
        "Syllabic",
        "Accentual",
    ];
    let figures = [
        "Metaphor",
        "Metonymy",
        "Synecdoche",
        "Apostrophe",
        "Enjambment",
        "Caesura",
        "Anaphora",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Poetry & Poetics" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tradition" }
                    select {
                        value: "{tradition}",
                        onchange: move |e| tradition.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in traditions { option { value: "{x}", "{x}" } }
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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Prosody" }
                    select {
                        value: "{prosody}",
                        onchange: move |e| prosody.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in prosodies { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Rhetorical Figure" }
                    select {
                        value: "{rhetorical_figure}",
                        onchange: move |e| rhetorical_figure.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in figures { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Poem Excerpt" }
                textarea {
                    value: "{poem_excerpt}",
                    oninput: move |e| poem_excerpt.set(e.value()),
                    placeholder: "Paste a poem or excerpt here for analysis...",
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{tradition} | {theoretical_approach} | {prosody} | {rhetorical_figure}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → poetics engine | prosody sieve | rhetorical anchor" }
            }
        }
    }
}
