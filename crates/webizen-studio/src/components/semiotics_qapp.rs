use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn SemioticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:semiotics".to_string(),
            title: "Semiotics Explorer".to_string()
        }
    }
}
