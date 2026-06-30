use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn EvolutionaryBiologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:evolutionarybiology".to_string(),
            title: "Evolutionary Biology Explorer".to_string()
        }
    }
}
