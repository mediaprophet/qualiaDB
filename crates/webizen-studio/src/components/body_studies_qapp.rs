use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn BodyStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:bodystudies".to_string(),
            title: "Body Studies Explorer".to_string()
        }
    }
}
