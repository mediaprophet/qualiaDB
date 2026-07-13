use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn GeophysicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:geophysics".to_string(),
            title: "Geophysics Explorer".to_string()
        }
    }
}
