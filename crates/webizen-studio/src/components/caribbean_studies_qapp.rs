use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn CaribbeanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:caribbeanstudies".to_string(),
            title: "Caribbean Studies Explorer".to_string()
        }
    }
}
