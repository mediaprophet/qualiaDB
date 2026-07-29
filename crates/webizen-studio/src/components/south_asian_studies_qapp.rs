use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn SouthAsianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:southasianstudies".to_string(),
            title: "South Asian Studies Explorer".to_string()
        }
    }
}
