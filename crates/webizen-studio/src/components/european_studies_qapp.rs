use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn EuropeanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:europeanstudies".to_string(),
            title: "European Studies Explorer".to_string()
        }
    }
}
