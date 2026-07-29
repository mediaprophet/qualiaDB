use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn OceanicAndPacificIslandStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:oceanicandpacificislandstudies".to_string(),
            title: "Oceanic And Pacific Island Studies Explorer".to_string()
        }
    }
}
