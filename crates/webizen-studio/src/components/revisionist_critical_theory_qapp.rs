use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn RevisionistCriticalTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:revisionistcriticaltheory".to_string(),
            title: "Revisionist Critical Theory Explorer".to_string()
        }
    }
}
