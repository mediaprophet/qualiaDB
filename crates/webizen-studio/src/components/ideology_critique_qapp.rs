use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn IdeologyCritiqueQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ideologycritique".to_string(),
            title: "Ideology Critique Explorer".to_string()
        }
    }
}
