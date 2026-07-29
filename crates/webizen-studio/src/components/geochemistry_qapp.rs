use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn GeochemistryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:geochemistry".to_string(),
            title: "Geochemistry Explorer".to_string()
        }
    }
}
