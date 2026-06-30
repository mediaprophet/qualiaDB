use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MediaEcologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:mediaecology".to_string(),
            title: "Media Ecology Explorer".to_string()
        }
    }
}
