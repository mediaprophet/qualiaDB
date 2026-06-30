use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn EconomicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:economics".to_string(),
            title: "Economics Explorer".to_string()
        }
    }
}
