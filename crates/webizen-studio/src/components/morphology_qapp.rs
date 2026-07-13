use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MorphologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:morphology".to_string(),
            title: "Morphology Explorer".to_string()
        }
    }
}
