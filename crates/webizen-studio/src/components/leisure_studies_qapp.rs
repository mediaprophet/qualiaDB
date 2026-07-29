use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn LeisureStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:leisurestudies".to_string(),
            title: "Leisure Studies Explorer".to_string()
        }
    }
}
