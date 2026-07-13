use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn IntermediaStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:intermediastudies".to_string(),
            title: "Intermedia Studies Explorer".to_string()
        }
    }
}
