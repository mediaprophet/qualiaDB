use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PhilosophyOfMindQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:philosophyofmind".to_string(),
            title: "Philosophy Of Mind Explorer".to_string()
        }
    }
}
