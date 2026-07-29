use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn RaceCriticalTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:racecriticaltheory".to_string(),
            title: "Race Critical Theory Explorer".to_string()
        }
    }
}
