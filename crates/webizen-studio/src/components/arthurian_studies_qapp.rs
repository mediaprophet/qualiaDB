use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn ArthurianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:arthurianstudies".to_string(),
            title: "Arthurian Studies Explorer".to_string()
        }
    }
}
