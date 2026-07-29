use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn CulturalEcologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:culturalecology".to_string(),
            title: "Cultural Ecology Explorer".to_string()
        }
    }
}
