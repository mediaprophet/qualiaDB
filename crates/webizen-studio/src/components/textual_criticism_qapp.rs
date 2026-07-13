use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn TextualCriticismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:textualcriticism".to_string(),
            title: "Textual Criticism Explorer".to_string()
        }
    }
}
