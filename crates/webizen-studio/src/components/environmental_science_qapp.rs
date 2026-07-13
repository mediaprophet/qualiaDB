use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn EnvironmentalScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:environmentalscience".to_string(),
            title: "Environmental Science Explorer".to_string()
        }
    }
}
