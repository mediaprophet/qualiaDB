use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn RevisionistCriticalTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:revisionistcriticaltheory".to_string(),
            title: "Revisionist Critical Theory Explorer".to_string()
        }
    }
}
