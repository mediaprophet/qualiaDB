use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PhilosophyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:philosophy".to_string(),
            title: "Philosophy Explorer".to_string()
        }
    }
}
