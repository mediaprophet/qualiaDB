use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PhilologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:philology".to_string(),
            title: "Philology Explorer".to_string()
        }
    }
}
