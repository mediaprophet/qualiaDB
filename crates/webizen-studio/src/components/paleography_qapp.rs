use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PaleographyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:paleography".to_string(),
            title: "Paleography Explorer".to_string()
        }
    }
}
