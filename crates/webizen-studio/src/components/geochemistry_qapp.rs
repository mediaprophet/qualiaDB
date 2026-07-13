use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn GeochemistryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:geochemistry".to_string(),
            title: "Geochemistry Explorer".to_string()
        }
    }
}
