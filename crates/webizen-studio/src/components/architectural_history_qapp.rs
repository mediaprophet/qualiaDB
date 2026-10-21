use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `ArchitecturalHistoryQapp` component.
pub fn ArchitecturalHistoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:architecturalhistory".to_string(),
            title: "Architectural History Explorer".to_string()
        }
    }
}
