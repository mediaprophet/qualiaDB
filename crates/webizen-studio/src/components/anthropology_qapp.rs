use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn AnthropologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:anthropology".to_string(),
            title: "Anthropology Explorer".to_string()
        }
    }
}
