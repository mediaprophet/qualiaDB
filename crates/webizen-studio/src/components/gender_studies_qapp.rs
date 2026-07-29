use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn GenderStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:genderstudies".to_string(),
            title: "Gender Studies Explorer".to_string()
        }
    }
}
