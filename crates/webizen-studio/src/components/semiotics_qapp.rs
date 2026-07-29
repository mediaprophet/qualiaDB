use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn SemioticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:semiotics".to_string(),
            title: "Semiotics Explorer".to_string()
        }
    }
}
