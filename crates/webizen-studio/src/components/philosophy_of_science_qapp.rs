use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn PhilosophyOfScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:philosophyofscience".to_string(),
            title: "Philosophy Of Science Explorer".to_string()
        }
    }
}
