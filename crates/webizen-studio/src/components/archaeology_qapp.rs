use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn ArchaeologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:archaeology".to_string(),
            title: "Archaeology Explorer".to_string()
        }
    }
}
