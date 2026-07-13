use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn SouthAsianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:southasianstudies".to_string(),
            title: "South Asian Studies Explorer".to_string()
        }
    }
}
