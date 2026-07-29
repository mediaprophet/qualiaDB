use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn PsychologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:psychology".to_string(),
            title: "Psychology Explorer".to_string()
        }
    }
}
