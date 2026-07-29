use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn WhitenessStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:whitenessstudies".to_string(),
            title: "Whiteness Studies Explorer".to_string()
        }
    }
}
