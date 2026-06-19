use dioxus::prelude::*;

#[component]
pub fn LatinAmericanStudiesQapp() -> Element {
    let mut subregion = use_signal(|| "Andes".to_string());
    let mut period = use_signal(|| "Contemporary".to_string());
    let mut disciplinary_lens = use_signal(|| "History".to_string());
    let mut language = use_signal(|| "Spanish".to_string());
    let mut theoretical_framework = use_signal(|| "Decolonial".to_string());
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Latin American Studies QApp" }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Sub-region" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| subregion.set(e.value()),
                    option { "Mexico" }
                    option { "Central America" }
                    option { "Caribbean" }
                    option { selected: true, "Andes" }
                    option { "Rio de la Plata" }
                    option { "Brazil" }
                    option { "Amazon" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Period" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| period.set(e.value()),
                    option { "Pre-Columbian" }
                    option { "Colonial" }
                    option { "Independence" }
                    option { "19th C." }
                    option { "Populist Era" }
                    option { "Cold War" }
                    option { "Neoliberal" }
                    option { "Pink Tide" }
                    option { selected: true, "Contemporary" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Disciplinary Lens" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| disciplinary_lens.set(e.value()),
                    option { selected: true, "History" }
                    option { "Literature" }
                    option { "Political Economy" }
                    option { "Anthropology" }
                    option { "Cultural Studies" }
                    option { "Postcolonial" }
                    option { "Migration" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Language" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| language.set(e.value()),
                    option { selected: true, "Spanish" }
                    option { "Portuguese" }
                    option { "Indigenous" }
                    option { "Creole" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Theoretical Framework" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| theoretical_framework.set(e.value()),
                    option { "Dependency Theory" }
                    option { "Liberation Theology" }
                    option { selected: true, "Decolonial" }
                    option { "Marxist" }
                    option { "Feminist" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box; min-height: 70px; resize: vertical;",
                    placeholder: "Research notes...",
                    oninput: move |e| notes.set(e.value()),
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "QualiaDB Output" }
                div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); display: flex; flex-direction: column; gap: 4px;",
                    div { "Sub-region: {subregion()}" }
                    div { "Period: {period()}" }
                    div { "Framework: {theoretical_framework()}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → knowledge graph | Allen Interval | geospatial sieve" }
            }
        }
    }
}
