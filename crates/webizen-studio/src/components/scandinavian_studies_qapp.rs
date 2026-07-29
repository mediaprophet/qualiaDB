use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn ScandinavianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:scandinavianstudies".to_string(),
            title: "Scandinavian Studies Explorer".to_string()
        }
    }
}
