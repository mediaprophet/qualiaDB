use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn LogicQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:logic".to_string(),
            title: "Logic Explorer".to_string()
        }
    }
}
