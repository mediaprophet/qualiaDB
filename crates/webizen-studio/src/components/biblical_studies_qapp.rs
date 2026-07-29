use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn BiblicalStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:biblicalstudies".to_string(),
            title: "Biblical Studies Explorer".to_string()
        }
    }
}
