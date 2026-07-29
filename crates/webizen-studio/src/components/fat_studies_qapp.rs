use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn FatStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:fatstudies".to_string(),
            title: "Fat Studies Explorer".to_string()
        }
    }
}
