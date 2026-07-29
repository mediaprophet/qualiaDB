use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn ComputerScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:computerscience".to_string(),
            title: "Computer Science Explorer".to_string()
        }
    }
}
