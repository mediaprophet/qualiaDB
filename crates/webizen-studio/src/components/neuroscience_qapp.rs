use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn NeuroscienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:neuroscience".to_string(),
            title: "Neuroscience Explorer".to_string()
        }
    }
}
