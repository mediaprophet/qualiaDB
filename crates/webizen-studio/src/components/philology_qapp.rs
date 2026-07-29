use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn PhilologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:philology".to_string(),
            title: "Philology Explorer".to_string()
        }
    }
}
