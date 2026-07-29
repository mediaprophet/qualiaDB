use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn EconomicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:economics".to_string(),
            title: "Economics Explorer".to_string()
        }
    }
}
