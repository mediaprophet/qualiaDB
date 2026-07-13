use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PhenomenologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:phenomenology".to_string(),
            title: "Phenomenology Explorer".to_string()
        }
    }
}
