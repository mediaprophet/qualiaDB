use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CeramicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ceramics".to_string(),
            title: "Ceramics Explorer".to_string()
        }
    }
}
