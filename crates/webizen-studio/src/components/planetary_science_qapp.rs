use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn PlanetaryScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:planetaryscience".to_string(),
            title: "Planetary Science Explorer".to_string()
        }
    }
}
