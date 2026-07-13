use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn ZoologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:zoology".to_string(),
            title: "Zoology Explorer".to_string()
        }
    }
}
