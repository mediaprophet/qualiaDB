use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PlanetaryScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:planetaryscience".to_string(),
            title: "Planetary Science Explorer".to_string()
        }
    }
}
