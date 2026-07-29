use super::{display_name, string_list, text_field};
use crate::Route;
use dioxus::prelude::*;

#[component]
pub(super) fn LibraryOverview(
    documents: usize,
    ontologies: usize,
    models: usize,
    facts: u64,
) -> Element {
    rsx! {
        div {
            div { style: "display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:10px;margin-bottom:18px;",
                Metric { label: "Documents".to_string(), value: documents.to_string(), icon: "file-earmark-text".to_string() }
                Metric { label: "Ontologies".to_string(), value: ontologies.to_string(), icon: "diagram-3".to_string() }
                Metric { label: "AI models".to_string(), value: models.to_string(), icon: "cpu".to_string() }
                Metric { label: "Graph facts".to_string(), value: facts.to_string(), icon: "share".to_string() }
            }
            div { style: "display:grid;grid-template-columns:1.2fr .8fr;gap:12px;",
                section { style: "border:1px solid var(--qualia-border);border-radius:15px;padding:18px;background:var(--qualia-surface);",
                    div { style: "font-size:.68rem;color:var(--qualia-accent);font-weight:800;text-transform:uppercase;letter-spacing:.1em;", "Start with a source" }
                    h3 { style: "margin:7px 0;font-size:1rem;", "Turn information into something Webizen can reason with" }
                    p { style: "margin:0 0 14px;color:var(--qualia-text-muted);font-size:.75rem;line-height:1.55;", "Import a pathology report, invoice, photo, book, dataset, ontology, or model. Webizen records provenance, proposes a domain mapping, validates the graph, and makes it available to Anatomy and QApps." }
                    div { style: "display:flex;gap:7px;flex-wrap:wrap;",
                        span { style: "font-size:.68rem;padding:5px 8px;border-radius:999px;background:rgba(127,127,127,.08);", "Pathology → Health" }
                        span { style: "font-size:.68rem;padding:5px 8px;border-radius:999px;background:rgba(127,127,127,.08);", "Invoice → Finance" }
                        span { style: "font-size:.68rem;padding:5px 8px;border-radius:999px;background:rgba(127,127,127,.08);", "OWL / TTL → Ontology" }
                    }
                }
                section { style: "border:1px solid var(--qualia-border);border-radius:15px;padding:18px;background:linear-gradient(145deg,var(--qualia-accent-glow),var(--qualia-surface));",
                    div { style: "font-size:.68rem;color:var(--qualia-accent);font-weight:800;text-transform:uppercase;letter-spacing:.1em;", "Connected capability" }
                    h3 { style: "margin:7px 0;font-size:1rem;", "Anatomy + comorbidity" }
                    p { style: "margin:0 0 12px;color:var(--qualia-text-muted);font-size:.74rem;line-height:1.5;", "Health graph facts can be resolved through FHIR, LOINC, ICD and Anatomy concepts, then evaluated by the local comorbidity engine." }
                    Link { to: Route::AnatomyRoute {}, style: "font-size:.75rem;color:var(--qualia-accent);font-weight:800;text-decoration:none;", "Open Anatomy →" }
                }
            }
        }
    }
}

#[component]
fn Metric(label: String, value: String, icon: String) -> Element {
    rsx! {
        div { style: "border:1px solid var(--qualia-border);border-radius:13px;background:var(--qualia-surface);padding:13px;",
            sl-icon { "name": "{icon}", style: "color:var(--qualia-accent);font-size:1rem;" }
            div { style: "font-size:1.25rem;font-weight:820;margin-top:7px;", "{value}" }
            div { style: "font-size:.66rem;color:var(--qualia-text-muted);margin-top:2px;", "{label}" }
        }
    }
}

#[component]
pub(super) fn SemanticPipeline() -> Element {
    let steps = [
        ("1", "Source", "Original file and provenance"),
        ("2", "Extract", "Text, media and structure"),
        ("3", "Map", "Domain ontology concepts"),
        ("4", "Validate", "SHACL and graph integrity"),
        ("5", "Use", "Search, inference and QApps"),
    ];
    rsx! {
        div {
            div { style: "font-size:.68rem;color:var(--qualia-accent);font-weight:800;text-transform:uppercase;letter-spacing:.1em;", "Semantic processing" }
            h3 { style: "font-size:1rem;margin:7px 0 5px;", "How an import becomes useful" }
            p { style: "font-size:.71rem;line-height:1.5;color:var(--qualia-text-muted);margin:0 0 17px;", "Nothing is silently treated as a pile of text. You can review the proposed meaning before it affects inference." }
            div { style: "display:flex;flex-direction:column;gap:8px;",
                for (number, title, detail) in steps {
                    div { style: "display:flex;gap:10px;align-items:flex-start;",
                        span { style: "width:24px;height:24px;border-radius:8px;background:var(--qualia-accent-glow);color:var(--qualia-accent);display:grid;place-items:center;font-size:.65rem;font-weight:850;flex:0 0 auto;", "{number}" }
                        div {
                            div { style: "font-size:.74rem;font-weight:760;", "{title}" }
                            div { style: "font-size:.66rem;color:var(--qualia-text-muted);margin-top:2px;", "{detail}" }
                        }
                    }
                }
            }
            div { style: "margin-top:20px;padding:12px;border:1px solid var(--qualia-border);border-radius:12px;",
                div { style: "font-size:.72rem;font-weight:760;", "Need raw graph controls?" }
                p { style: "font-size:.66rem;line-height:1.45;color:var(--qualia-text-muted);margin:4px 0 0;", "Switch to Advanced Technical for CML, COF, observer rights, spatial morphs, facets, and graph export." }
            }
        }
    }
}

#[component]
pub(super) fn ItemInspector(item: serde_json::Value) -> Element {
    let topics = string_list(&item, "topics");
    rsx! {
        div {
            div { style: "font-size:.68rem;color:var(--qualia-accent);font-weight:800;text-transform:uppercase;letter-spacing:.1em;margin-bottom:12px;", "Selected item" }
            h3 { style: "font-size:1rem;margin:0 0 5px;word-break:break-word;", "{display_name(&item)}" }
            div { style: "font-size:.67rem;color:var(--qualia-text-muted);word-break:break-all;", {text_field(&item, "asset_uri")} }
            div { style: "margin:16px 0;border-top:1px solid var(--qualia-border);" }
            InspectorField { label: "Media type".to_string(), value: text_field(&item, "media_type") }
            InspectorField { label: "Library section".to_string(), value: text_field(&item, "section") }
            InspectorField { label: "Sensitivity".to_string(), value: text_field(&item, "sensitivity") }
            div { style: "font-size:.65rem;color:var(--qualia-text-muted);margin-bottom:6px;", "Ontology concepts / topics" }
            div { style: "display:flex;gap:5px;flex-wrap:wrap;",
                for topic in topics {
                    span { style: "font-size:.64rem;padding:4px 7px;border-radius:999px;background:var(--qualia-accent-glow);color:var(--qualia-accent);", "{topic}" }
                }
            }
            p { style: "font-size:.7rem;line-height:1.55;color:var(--qualia-text-muted);margin-top:16px;", {text_field(&item, "excerpt")} }
        }
    }
}

#[component]
fn InspectorField(label: String, value: String) -> Element {
    rsx! {
        div { style: "margin-bottom:12px;",
            div { style: "font-size:.64rem;color:var(--qualia-text-muted);margin-bottom:3px;", "{label}" }
            div { style: "font-size:.73rem;font-weight:690;word-break:break-word;", if value.is_empty() { "Not recorded" } else { "{value}" } }
        }
    }
}

#[component]
pub(super) fn EmptyCollection(collection: String) -> Element {
    rsx! {
        div { style: "border:1px dashed var(--qualia-border);border-radius:15px;padding:34px;text-align:center;background:rgba(127,127,127,.025);",
            sl-icon { "name": "inbox", style: "font-size:1.5rem;color:var(--qualia-text-muted);" }
            h3 { style: "font-size:.9rem;margin:10px 0 5px;", "No {collection} yet" }
            p { style: "font-size:.72rem;color:var(--qualia-text-muted);margin:0;", "Import a source or change collection. Webizen will show the proposed semantic mapping before graph conversion." }
        }
    }
}
