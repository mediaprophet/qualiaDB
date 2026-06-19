use dioxus::prelude::*;

#[component]
pub fn ScreenwritingQapp() -> Element {
    let mut format = use_signal(|| "Feature Film".to_string());
    let mut genre = use_signal(|| "Drama".to_string());
    let mut structure = use_signal(|| "Three-Act".to_string());
    let mut page_count = use_signal(|| 110u32);
    let mut act_break_page_1 = use_signal(|| 30u32);
    let mut act_break_page_2 = use_signal(|| 90u32);
    let mut logline = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    let formats = [
        "Feature Film",
        "Short Film",
        "TV Pilot",
        "TV Series",
        "Limited Series",
        "Web Series",
        "Documentary",
        "Video Game Narrative",
        "Interactive",
    ];
    let genres = [
        "Drama", "Comedy", "Thriller", "Horror", "Sci-Fi", "Action", "Romance", "Animated",
        "Hybrid",
    ];
    let structures = [
        "Three-Act",
        "Save the Cat",
        "Five-Act",
        "Non-Linear",
        "Episodic",
        "Anthology",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Screenwriting" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Format" }
                    select {
                        value: "{format}",
                        onchange: move |e| format.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in formats { option { value: "{x}", "{x}" } }
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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Structure" }
                    select {
                        value: "{structure}",
                        onchange: move |e| structure.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in structures { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Page Count: {page_count}" }
                input {
                    r#type: "range",
                    min: "1",
                    max: "200",
                    value: "{page_count}",
                    oninput: move |e| page_count.set(e.value().parse().unwrap_or(110)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Act Break Page 1: {act_break_page_1}" }
                    input {
                        r#type: "range",
                        min: "10",
                        max: "60",
                        value: "{act_break_page_1}",
                        oninput: move |e| act_break_page_1.set(e.value().parse().unwrap_or(30)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Act Break Page 2: {act_break_page_2}" }
                    input {
                        r#type: "range",
                        min: "60",
                        max: "120",
                        value: "{act_break_page_2}",
                        oninput: move |e| act_break_page_2.set(e.value().parse().unwrap_or(90)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Logline" }
                textarea {
                    value: "{logline}",
                    oninput: move |e| logline.set(e.value()),
                    placeholder: "A one-sentence summary of your screenplay...",
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{format} | {genre} | {structure} | {page_count}pp | Act breaks: p{act_break_page_1}/p{act_break_page_2}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → screenplay structure engine | genre sieve | story anchor" }
            }
        }
    }
}
