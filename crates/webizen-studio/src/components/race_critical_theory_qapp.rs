use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn RaceCriticalTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:racecriticaltheory".to_string(),
            title: "Race Critical Theory Explorer".to_string()
        }
    }
}
