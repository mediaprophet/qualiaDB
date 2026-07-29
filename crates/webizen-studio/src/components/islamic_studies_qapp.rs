use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn IslamicStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:islamicstudies".to_string(),
            title: "Islamic Studies Explorer".to_string()
        }
    }
}
