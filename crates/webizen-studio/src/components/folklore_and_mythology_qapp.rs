use dioxus::prelude::*;

#[component]
pub fn FolkloreAndMythologyQapp() -> Element {
    let mut tradition = use_signal(|| "Greek".to_string());
    let mut genre = use_signal(|| "Myth".to_string());
    let mut comparative_method = use_signal(|| "Structuralist".to_string());
    let mut motif_index = use_signal(|| String::new());
    let mut tale_type = use_signal(|| String::new());
    let mut cultural_context = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Folklore & Mythology QApp" }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Tradition" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| tradition.set(e.value()),
                    option { selected: true, "Greek" }
                    option { "Roman" }
                    option { "Norse" }
                    option { "Celtic" }
                    option { "Slavic" }
                    option { "Hindu" }
                    option { "Mesopotamian" }
                    option { "Egyptian" }
                    option { "Chinese" }
                    option { "Japanese" }
                    option { "Indigenous American" }
                    option { "African" }
                    option { "Pacific Islander" }
                    option { "Caribbean" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Genre" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| genre.set(e.value()),
                    option { selected: true, "Myth" }
                    option { "Legend" }
                    option { "Fairy Tale" }
                    option { "Epic" }
                    option { "Fable" }
                    option { "Trickster Tale" }
                    option { "Hero Cycle" }
                    option { "Cosmogony" }
                    option { "Eschatology" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Comparative Method" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| comparative_method.set(e.value()),
                    option { selected: true, "Structuralist" }
                    option { "Functional" }
                    option { "Historical-Geographical" }
                    option { "Psychoanalytic" }
                    option { "Jungian Archetype" }
                    option { "Proppian Morphology" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Motif Index" }
                input {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    placeholder: "e.g. H1234, A100...",
                    oninput: move |e| motif_index.set(e.value()),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Tale Type (ATU)" }
                input {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    placeholder: "e.g. ATU 300, ATU 510A...",
                    oninput: move |e| tale_type.set(e.value()),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Cultural Context" }
                textarea {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box; min-height: 70px; resize: vertical;",
                    placeholder: "Describe the cultural context...",
                    oninput: move |e| cultural_context.set(e.value()),
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "QualiaDB Output" }
                div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); display: flex; flex-direction: column; gap: 4px;",
                    div { "Tradition: {tradition()}" }
                    div { "Genre: {genre()}" }
                    div { "Method: {comparative_method()}" }
                    div { "Motif: {motif_index()}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → graph theory motif | neuro-symbolic archetype | Allen Interval" }
            }
        }
    }
}
