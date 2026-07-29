use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn DecolonialStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:decolonialstudies".to_string(),
            title: "Decolonial Studies Explorer".to_string()
        }
    }
}
