use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn SyntaxQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:syntax".to_string(),
            title: "Syntax Explorer".to_string()
        }
    }
}
