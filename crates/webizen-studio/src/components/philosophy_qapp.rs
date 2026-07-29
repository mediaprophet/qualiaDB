use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn PhilosophyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:philosophy".to_string(),
            title: "Philosophy Explorer".to_string()
        }
    }
}
