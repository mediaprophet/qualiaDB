use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn BuddhistStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:buddhiststudies".to_string(),
            title: "Buddhist Studies Explorer".to_string()
        }
    }
}
