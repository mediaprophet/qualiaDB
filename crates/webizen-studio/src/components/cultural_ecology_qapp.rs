use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CulturalEcologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:culturalecology".to_string(),
            title: "Cultural Ecology Explorer".to_string()
        }
    }
}
