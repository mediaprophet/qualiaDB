use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn DeafStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:deafstudies".to_string(),
            title: "Deaf Studies Explorer".to_string()
        }
    }
}
