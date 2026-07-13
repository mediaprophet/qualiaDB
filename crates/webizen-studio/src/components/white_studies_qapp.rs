use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn WhiteStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:whitestudies".to_string(),
            title: "White Studies Explorer".to_string()
        }
    }
}
