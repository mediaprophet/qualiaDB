use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MarineBiologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:marinebiology".to_string(),
            title: "Marine Biology Explorer".to_string()
        }
    }
}
