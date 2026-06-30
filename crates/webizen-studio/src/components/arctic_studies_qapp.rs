use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn ArcticStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:arcticstudies".to_string(),
            title: "Arctic Studies Explorer".to_string()
        }
    }
}
