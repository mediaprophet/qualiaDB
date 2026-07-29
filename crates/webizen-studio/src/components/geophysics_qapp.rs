use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn GeophysicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:geophysics".to_string(),
            title: "Geophysics Explorer".to_string()
        }
    }
}
