use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn BiopoliticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:biopolitics".to_string(),
            title: "Biopolitics Explorer".to_string()
        }
    }
}
