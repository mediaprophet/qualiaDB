use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn AestheticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:aesthetics".to_string(),
            title: "Aesthetics Explorer".to_string()
        }
    }
}
