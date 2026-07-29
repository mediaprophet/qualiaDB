use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn HinduStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:hindustudies".to_string(),
            title: "Hindu Studies Explorer".to_string()
        }
    }
}
