use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn GriefStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:griefstudies".to_string(),
            title: "Grief Studies Explorer".to_string()
        }
    }
}
