use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn GeologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:geology".to_string(),
            title: "Geology Explorer".to_string()
        }
    }
}
