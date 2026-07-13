use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn AppalachianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:appalachianstudies".to_string(),
            title: "Appalachian Studies Explorer".to_string()
        }
    }
}
