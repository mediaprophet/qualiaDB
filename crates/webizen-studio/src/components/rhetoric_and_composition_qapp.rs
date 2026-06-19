use dioxus::prelude::*;

#[component]
pub fn RhetoricAndCompositionQapp() -> Element {
    let mut rhetorical_tradition = use_signal(|| "Classical Greek".to_string());
    let mut appeal = use_signal(|| "Logos".to_string());
    let mut genre = use_signal(|| "Deliberative".to_string());
    let mut discourse_mode = use_signal(|| "Argument".to_string());
    let mut stasis_question = use_signal(|| "Fact".to_string());
    let mut text_excerpt = use_signal(|| String::new());
    let mut compositional_notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Rhetoric & Composition QApp" }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Rhetorical Tradition" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| rhetorical_tradition.set(e.value()),
                    option { selected: true, "Classical Greek" }
                    option { "Roman" }
                    option { "Medieval" }
                    option { "Enlightenment" }
                    option { "19th C. Elocution" }
                    option { "Contemporary" }
                    option { "Digital" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Rhetorical Appeal" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| appeal.set(e.value()),
                    option { "Ethos" }
                    option { "Pathos" }
                    option { selected: true, "Logos" }
                    option { "Kairos" }
                    option { "Telos" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Genre" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| genre.set(e.value()),
                    option { selected: true, "Deliberative" }
                    option { "Forensic" }
                    option { "Epideictic" }
                    option { "Academic" }
                    option { "Public" }
                    option { "Digital" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Discourse Mode" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| discourse_mode.set(e.value()),
                    option { selected: true, "Argument" }
                    option { "Narration" }
                    option { "Description" }
                    option { "Exposition" }
                    option { "Comparison" }
                    option { "Definition" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Stasis Question" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| stasis_question.set(e.value()),
                    option { selected: true, "Fact" }
                    option { "Definition" }
                    option { "Quality" }
                    option { "Procedure" }
                    option { "Jurisdiction" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Text Excerpt" }
                textarea {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box; min-height: 70px; resize: vertical;",
                    placeholder: "Paste text excerpt for analysis...",
                    oninput: move |e| text_excerpt.set(e.value()),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Compositional Notes" }
                textarea {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box; min-height: 60px; resize: vertical;",
                    placeholder: "Compositional analysis notes...",
                    oninput: move |e| compositional_notes.set(e.value()),
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "QualiaDB Output" }
                div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); display: flex; flex-direction: column; gap: 4px;",
                    div { "Tradition: {rhetorical_tradition()}" }
                    div { "Appeal: {appeal()}" }
                    div { "Genre: {genre()}" }
                    div { "Stasis: {stasis_question()}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → discourse sieve | epistemic logic | neuro-symbolic" }
            }
        }
    }
}
