use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn AtmosphericScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:atmosphericscience".to_string(),
            title: "Atmospheric Science Explorer".to_string()
        }
    }
}
