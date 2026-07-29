use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn MemoryStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:memorystudies".to_string(),
            title: "Memory Studies Explorer".to_string()
        }
    }
}
