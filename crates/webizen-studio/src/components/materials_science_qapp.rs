use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MaterialsScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:materialsscience".to_string(),
            title: "Materials Science Explorer".to_string()
        }
    }
}
