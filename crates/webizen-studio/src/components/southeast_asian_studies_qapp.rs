use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn SoutheastAsianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:southeastasianstudies".to_string(),
            title: "Southeast Asian Studies Explorer".to_string()
        }
    }
}
