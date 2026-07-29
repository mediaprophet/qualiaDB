use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn PrintmakingQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:printmaking".to_string(),
            title: "Printmaking Explorer".to_string()
        }
    }
}
