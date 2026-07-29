use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn CulturalStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:culturalstudies".to_string(),
            title: "Cultural Studies Explorer".to_string()
        }
    }
}
