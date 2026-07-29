use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn TextualCriticismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:textualcriticism".to_string(),
            title: "Textual Criticism Explorer".to_string()
        }
    }
}
