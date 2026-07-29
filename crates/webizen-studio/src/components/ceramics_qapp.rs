use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn CeramicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ceramics".to_string(),
            title: "Ceramics Explorer".to_string()
        }
    }
}
