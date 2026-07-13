use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PostcolonialStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:postcolonialstudies".to_string(),
            title: "Postcolonial Studies Explorer".to_string()
        }
    }
}
