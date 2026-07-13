use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn LinguisticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:linguistics".to_string(),
            title: "Linguistics Explorer".to_string()
        }
    }
}
