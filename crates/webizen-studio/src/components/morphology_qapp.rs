use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn MorphologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:morphology".to_string(),
            title: "Morphology Explorer".to_string()
        }
    }
}
