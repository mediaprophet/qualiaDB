use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CognitiveScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:cognitivescience".to_string(),
            title: "Cognitive Science Explorer".to_string()
        }
    }
}
