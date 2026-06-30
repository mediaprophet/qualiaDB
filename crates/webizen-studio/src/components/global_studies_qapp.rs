use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn GlobalStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:globalstudies".to_string(),
            title: "Global Studies Explorer".to_string()
        }
    }
}
