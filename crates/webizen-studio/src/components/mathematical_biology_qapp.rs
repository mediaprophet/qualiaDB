use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn MathematicalBiologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:mathematicalbiology".to_string(),
            title: "Mathematical Biology Explorer".to_string()
        }
    }
}
