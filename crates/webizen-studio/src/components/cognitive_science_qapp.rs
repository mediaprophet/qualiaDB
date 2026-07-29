use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn CognitiveScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:cognitivescience".to_string(),
            title: "Cognitive Science Explorer".to_string()
        }
    }
}
