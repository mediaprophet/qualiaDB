use dioxus::prelude::*;

#[component]
pub fn IndigenousAndNativeAmericanStudiesQapp() -> Element {
    let mut geographic_region = use_signal(|| "Northeast Woodlands".to_string());
    let mut disciplinary_lens = use_signal(|| "Sovereignty & Legal".to_string());
    let mut community_partnership_mode = use_signal(|| "Collaborative".to_string());
    let mut temporal_focus = use_signal(|| String::new());
    let mut nation_or_people = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Indigenous & Native American Studies QApp" }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Geographic Region" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| geographic_region.set(e.value()),
                    option { selected: true, "Northeast Woodlands" }
                    option { "Southeast" }
                    option { "Great Plains" }
                    option { "Southwest" }
                    option { "Pacific Northwest" }
                    option { "Arctic" }
                    option { "Great Basin" }
                    option { "Mesoamerica" }
                    option { "Andean" }
                    option { "Amazonian" }
                    option { "Pacific Islands" }
                    option { "Australia" }
                    option { "Aotearoa NZ" }
                    option { "Other" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Disciplinary Lens" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| disciplinary_lens.set(e.value()),
                    option { selected: true, "Sovereignty & Legal" }
                    option { "Language Revitalisation" }
                    option { "Environmental Knowledge" }
                    option { "Cultural Continuity" }
                    option { "Oral History" }
                    option { "Postcolonial" }
                    option { "Decolonial" }
                    option { "Material Culture" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Community Partnership Mode" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| community_partnership_mode.set(e.value()),
                    option { "Community-Led" }
                    option { selected: true, "Collaborative" }
                    option { "Academic-Led" }
                    option { "Archival Only" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Temporal Focus" }
                input {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    placeholder: "e.g. Pre-contact, Reservation Era, Contemporary...",
                    oninput: move |e| temporal_focus.set(e.value()),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Nation or People" }
                input {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    placeholder: "e.g. Haudenosaunee, Lakota, Māori...",
                    oninput: move |e| nation_or_people.set(e.value()),
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
                    div { "Region: {geographic_region()}" }
                    div { "Lens: {disciplinary_lens()}" }
                    div { "Partnership: {community_partnership_mode()}" }
                    div { "People: {nation_or_people()}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → knowledge graph | provenance | Allen Interval" }
            }
        }
    }
}
