use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn SemanticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:semantics".to_string(),
            title: "Semantics Explorer".to_string()
        }
    }
}
