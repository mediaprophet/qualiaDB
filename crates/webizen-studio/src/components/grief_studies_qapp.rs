use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn GriefStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:griefstudies".to_string(),
            title: "Grief Studies Explorer".to_string()
        }
    }
}
