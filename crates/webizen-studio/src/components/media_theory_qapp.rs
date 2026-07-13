use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MediaTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:mediatheory".to_string(),
            title: "Media Theory Explorer".to_string()
        }
    }
}
