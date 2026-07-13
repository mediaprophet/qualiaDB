use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn RaceTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:racetheory".to_string(),
            title: "Race Theory Explorer".to_string()
        }
    }
}
