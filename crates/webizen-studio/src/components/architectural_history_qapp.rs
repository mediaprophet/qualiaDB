use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn ArchitecturalHistoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:architecturalhistory".to_string(),
            title: "Architectural History Explorer".to_string()
        }
    }
}
