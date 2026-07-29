use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn LinguisticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:linguistics".to_string(),
            title: "Linguistics Explorer".to_string()
        }
    }
}
