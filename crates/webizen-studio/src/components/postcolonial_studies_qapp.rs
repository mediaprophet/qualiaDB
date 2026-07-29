use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn PostcolonialStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:postcolonialstudies".to_string(),
            title: "Postcolonial Studies Explorer".to_string()
        }
    }
}
