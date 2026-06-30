use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn FanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:fanstudies".to_string(),
            title: "Fan Studies Explorer".to_string()
        }
    }
}
