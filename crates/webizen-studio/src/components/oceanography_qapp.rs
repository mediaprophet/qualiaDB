use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn OceanographyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:oceanography".to_string(),
            title: "Oceanography Explorer".to_string()
        }
    }
}
