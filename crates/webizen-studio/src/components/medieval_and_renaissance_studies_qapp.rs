use dioxus::prelude::*;

#[component]
pub fn MedievalAndRenaissanceStudiesQapp() -> Element {
    let mut period = use_signal(|| "High Medieval 1000-1300".to_string());
    let mut geographic_focus = use_signal(|| "Italy".to_string());
    let mut discipline = use_signal(|| "History".to_string());
    let mut language = use_signal(|| "Latin".to_string());
    let mut manuscript_reference = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Medieval & Renaissance Studies QApp" }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Period" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| period.set(e.value()),
                    option { "Early Medieval 500-900" }
                    option { "Carolingian" }
                    option { selected: true, "High Medieval 1000-1300" }
                    option { "Late Medieval 1300-1500" }
                    option { "Early Renaissance" }
                    option { "High Renaissance" }
                    option { "Northern Renaissance" }
                    option { "Mannerism" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Geographic Focus" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| geographic_focus.set(e.value()),
                    option { "British Isles" }
                    option { "France" }
                    option { selected: true, "Italy" }
                    option { "Holy Roman Empire" }
                    option { "Iberia" }
                    option { "Byzantine" }
                    option { "Islamic World" }
                    option { "Scandinavia" }
                    option { "Eastern Europe" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Discipline" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| discipline.set(e.value()),
                    option { selected: true, "History" }
                    option { "Literature" }
                    option { "Art History" }
                    option { "Music" }
                    option { "Philosophy" }
                    option { "Theology" }
                    option { "Science" }
                    option { "Law" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Language" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| language.set(e.value()),
                    option { selected: true, "Latin" }
                    option { "Old French" }
                    option { "Middle English" }
                    option { "Middle High German" }
                    option { "Italian" }
                    option { "Arabic" }
                    option { "Greek" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Manuscript Reference" }
                input {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    placeholder: "e.g. BnF MS lat. 1234, Cotton Nero A.x...",
                    oninput: move |e| manuscript_reference.set(e.value()),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box; min-height: 60px; resize: vertical;",
                    placeholder: "Additional notes...",
                    oninput: move |e| notes.set(e.value()),
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "QualiaDB Output" }
                div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); display: flex; flex-direction: column; gap: 4px;",
                    div { "Period: {period()}" }
                    div { "Region: {geographic_focus()}" }
                    div { "Discipline: {discipline()}" }
                    div { "Language: {language()}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → Allen Interval | textual sieve | knowledge graph" }
            }
        }
    }
}
