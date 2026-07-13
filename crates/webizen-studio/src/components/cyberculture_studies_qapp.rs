use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CybercultureStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:cyberculturestudies".to_string(),
            title: "Cyberculture Studies Explorer".to_string()
        }
    }
}
