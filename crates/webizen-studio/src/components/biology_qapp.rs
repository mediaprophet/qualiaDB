use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn BiologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:biology".to_string(),
            title: "Biology Explorer".to_string()
        }
    }
}
