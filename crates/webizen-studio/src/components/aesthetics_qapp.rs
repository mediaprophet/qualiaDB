use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn AestheticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:aesthetics".to_string(),
            title: "Aesthetics Explorer".to_string()
        }
    }
}
