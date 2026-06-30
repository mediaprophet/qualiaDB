use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PhotographyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:photography".to_string(),
            title: "Photography Explorer".to_string()
        }
    }
}
