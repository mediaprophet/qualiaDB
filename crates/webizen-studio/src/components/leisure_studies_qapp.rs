use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn LeisureStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:leisurestudies".to_string(),
            title: "Leisure Studies Explorer".to_string()
        }
    }
}
