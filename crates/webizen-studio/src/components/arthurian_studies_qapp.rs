use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn ArthurianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:arthurianstudies".to_string(),
            title: "Arthurian Studies Explorer".to_string()
        }
    }
}
