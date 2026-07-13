use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn WhitenessStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:whitenessstudies".to_string(),
            title: "Whiteness Studies Explorer".to_string()
        }
    }
}
