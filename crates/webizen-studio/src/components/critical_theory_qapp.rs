use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CriticalTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:criticaltheory".to_string(),
            title: "Critical Theory Explorer".to_string()
        }
    }
}
