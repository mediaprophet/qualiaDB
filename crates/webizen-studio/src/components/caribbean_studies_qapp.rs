use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CaribbeanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:caribbeanstudies".to_string(),
            title: "Caribbean Studies Explorer".to_string()
        }
    }
}
