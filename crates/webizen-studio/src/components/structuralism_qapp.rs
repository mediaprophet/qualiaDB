use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn StructuralismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:structuralism".to_string(),
            title: "Structuralism Explorer".to_string()
        }
    }
}
