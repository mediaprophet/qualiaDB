use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CelticStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:celticstudies".to_string(),
            title: "Celtic Studies Explorer".to_string()
        }
    }
}
