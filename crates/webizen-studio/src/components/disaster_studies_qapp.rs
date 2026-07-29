use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn DisasterStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:disasterstudies".to_string(),
            title: "Disaster Studies Explorer".to_string()
        }
    }
}
