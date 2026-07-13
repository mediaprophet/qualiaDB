use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn OceanographyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:oceanography".to_string(),
            title: "Oceanography Explorer".to_string()
        }
    }
}
