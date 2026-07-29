use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn SemanticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:semantics".to_string(),
            title: "Semantics Explorer".to_string()
        }
    }
}
