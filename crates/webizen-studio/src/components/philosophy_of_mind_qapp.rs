use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `PhilosophyOfMindQapp` component.
pub fn PhilosophyOfMindQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:philosophyofmind".to_string(),
            title: "Philosophy Of Mind Explorer".to_string()
        }
    }
}
