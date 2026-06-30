use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn ScandinavianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:scandinavianstudies".to_string(),
            title: "Scandinavian Studies Explorer".to_string()
        }
    }
}
