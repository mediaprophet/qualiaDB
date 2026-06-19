use dioxus::prelude::*;

#[component]
pub fn MuseumStudiesQapp() -> Element {
    let mut museum_type = use_signal(|| "Art".to_string());
    let mut collection_type = use_signal(|| "Artefact".to_string());
    let mut curation_approach = use_signal(|| "Thematic".to_string());
    let mut interpretation_method = use_signal(|| "Digital Interactive".to_string());
    let mut provenance_certainty = use_signal(|| 75u32);
    let mut access_model = use_signal(|| "Mixed".to_string());
    let mut conservation_priority = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Museum Studies QApp" }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Museum Type" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| museum_type.set(e.value()),
                    option { selected: true, "Art" }
                    option { "Natural History" }
                    option { "Science & Technology" }
                    option { "History" }
                    option { "Ethnographic" }
                    option { "Living History" }
                    option { "Digital" }
                    option { "Community" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Collection Type" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| collection_type.set(e.value()),
                    option { selected: true, "Artefact" }
                    option { "Specimen" }
                    option { "Digital Object" }
                    option { "Born-Digital" }
                    option { "Archival Material" }
                    option { "Living Collection" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Curation Approach" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| curation_approach.set(e.value()),
                    option { "Encyclopaedic" }
                    option { selected: true, "Thematic" }
                    option { "Narrative" }
                    option { "Immersive" }
                    option { "Decolonial" }
                    option { "Community-Co-curated" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Interpretation Method" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| interpretation_method.set(e.value()),
                    option { "Label" }
                    option { "Audio Guide" }
                    option { selected: true, "Digital Interactive" }
                    option { "AR" }
                    option { "VR" }
                    option { "Haptic" }
                    option { "Performance" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Provenance Certainty (0–100): {provenance_certainty()}%" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "100",
                    step: "1",
                    value: "{provenance_certainty()}",
                    style: "width: 100%; box-sizing: border-box; accent-color: if provenance_certainty() >= 70 { \"var(--qualia-accent)\" } else { \"var(--qualia-accent)\" };",
                    oninput: move |e| provenance_certainty.set(e.value().parse().unwrap_or(75)),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Access Model" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| access_model.set(e.value()),
                    option { "Free" }
                    option { "Ticketed" }
                    option { selected: true, "Mixed" }
                    option { "Online-Only" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Conservation Priority" }
                input {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    placeholder: "e.g. climate control, digitisation, repatriation...",
                    oninput: move |e| conservation_priority.set(e.value()),
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
                    div { "Type: {museum_type()}" }
                    div { "Curation: {curation_approach()}" }
                    div { "Interpretation: {interpretation_method()}" }
                    div { style: "color: if provenance_certainty() >= 70 { \"var(--qualia-accent)\" } else { \"var(--qualia-accent)\" };", "Provenance: {provenance_certainty()}%" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → provenance graph | knowledge graph | SPARQL" }
            }
        }
    }
}
