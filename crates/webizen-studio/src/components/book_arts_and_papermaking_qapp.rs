use dioxus::prelude::*;

#[component]
pub fn BookArtsAndPapermakingQapp() -> Element {
    let mut technique = use_signal(|| "Letterpress".to_string());
    let mut paper_type = use_signal(|| "Japanese Washi".to_string());
    let mut binding_style = use_signal(|| "Codex".to_string());
    let mut edition_size = use_signal(|| 50u32);
    let mut historical_period = use_signal(|| "Modern".to_string());
    let mut notes = use_signal(|| String::new());

    let techniques = [
        "Hand Papermaking",
        "Screen Printing",
        "Letterpress",
        "Bookbinding",
        "Marbling",
        "Calligraphy",
        "Illumination",
        "Artists Book",
        "Zine",
        "Risograph",
    ];
    let paper_types = [
        "Japanese Washi",
        "Handmade Rag",
        "Cotton Rag",
        "Recycled",
        "Vellum",
        "Parchment",
    ];
    let binding_styles = [
        "Codex",
        "Japanese",
        "Coptic",
        "Ethiopian",
        "Longstitch",
        "Accordion",
        "Drum Leaf",
    ];
    let periods = [
        "Ancient Scroll",
        "Medieval Manuscript",
        "Early Print",
        "Modern",
        "Contemporary",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Book Arts & Papermaking" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Technique" }
                    select {
                        value: "{technique}",
                        onchange: move |e| technique.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in techniques { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Paper Type" }
                    select {
                        value: "{paper_type}",
                        onchange: move |e| paper_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in paper_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Binding Style" }
                    select {
                        value: "{binding_style}",
                        onchange: move |e| binding_style.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in binding_styles { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Historical Period" }
                    select {
                        value: "{historical_period}",
                        onchange: move |e| historical_period.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in periods { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Edition Size: {edition_size}" }
                input {
                    r#type: "range",
                    min: "1",
                    max: "1000",
                    value: "{edition_size}",
                    oninput: move |e| edition_size.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{technique} | {paper_type} | {binding_style} | Edition: {edition_size} | {historical_period}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → book arts engine | material culture sieve | edition anchor" }
            }
        }
    }
}
