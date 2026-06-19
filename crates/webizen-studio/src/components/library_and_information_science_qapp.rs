use dioxus::prelude::*;

#[component]
pub fn LibraryAndInformationScienceQapp() -> Element {
    let mut subfield = use_signal(|| "Metadata".to_string());
    let mut metadata_schema = use_signal(|| "Dublin Core".to_string());
    let mut classification_scheme = use_signal(|| "Dewey".to_string());
    let mut information_type = use_signal(|| "Text".to_string());
    let mut access_model = use_signal(|| "Open Access".to_string());
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Library & Information Science QApp" }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Subfield" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| subfield.set(e.value()),
                    option { "Cataloguing" }
                    option { selected: true, "Metadata" }
                    option { "Archival Science" }
                    option { "Digital Preservation" }
                    option { "Knowledge Management" }
                    option { "Information Retrieval" }
                    option { "User Experience" }
                    option { "Data Curation" }
                    option { "Community Librarianship" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Metadata Schema" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| metadata_schema.set(e.value()),
                    option { selected: true, "Dublin Core" }
                    option { "MARC21" }
                    option { "MODS" }
                    option { "RDF" }
                    option { "Schema.org" }
                    option { "EAD" }
                    option { "TEI" }
                    option { "PREMIS" }
                    option { "JSON-LD" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Classification Scheme" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| classification_scheme.set(e.value()),
                    option { selected: true, "Dewey" }
                    option { "LCC" }
                    option { "UDC" }
                    option { "BISAC" }
                    option { "Bliss" }
                    option { "Colon Classification" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Information Type" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| information_type.set(e.value()),
                    option { selected: true, "Text" }
                    option { "Image" }
                    option { "Audio" }
                    option { "Video" }
                    option { "Dataset" }
                    option { "3D Object" }
                    option { "Born-Digital" }
                    option { "Physical" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Access Model" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| access_model.set(e.value()),
                    option { selected: true, "Open Access" }
                    option { "Paywalled" }
                    option { "Restricted" }
                    option { "Dark Archive" }
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
                    div { "Subfield: {subfield()}" }
                    div { "Schema: {metadata_schema()}" }
                    div { "Classification: {classification_scheme()}" }
                    div { "Access: {access_model()}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → knowledge graph | RDF engine | SPARQL explorer" }
            }
        }
    }
}
