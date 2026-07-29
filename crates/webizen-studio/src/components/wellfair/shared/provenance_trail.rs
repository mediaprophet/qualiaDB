use super::super::host_dto::ProvenanceHop;
use dioxus::prelude::*;

#[component]
pub fn ProvenanceTrail(hops: Vec<ProvenanceHop>) -> Element {
    if hops.is_empty() {
        return rsx! {
            p { style: "font-size:0.8rem;color:var(--qualia-text-muted,#666);", "No provenance chain supplied by host." }
        };
    }

    rsx! {
        ol {
            style: "margin:0;padding-left:1.1rem;display:grid;gap:0.35rem;",
            aria_label: "Provenance trail",
            for (idx, hop) in hops.iter().enumerate() {
                li {
                    key: "{idx}",
                    style: "font-size:0.8rem;line-height:1.35;",
                    strong { "{hop.label}" }
                    span { style: "color:var(--qualia-text-muted,#666);", " · {hop.evidence_type} · {hop.hash_prefix}" }
                }
            }
        }
    }
}
