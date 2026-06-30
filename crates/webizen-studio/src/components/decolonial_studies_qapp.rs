use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn DecolonialStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:decolonialstudies".to_string(),
            title: "Decolonial Studies Explorer".to_string()
        }
    }
}
