use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CulturalStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:culturalstudies".to_string(),
            title: "Cultural Studies Explorer".to_string()
        }
    }
}
