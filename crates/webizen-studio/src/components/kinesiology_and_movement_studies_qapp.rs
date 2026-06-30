use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn KinesiologyAndMovementStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:kinesiologyandmovementstudies".to_string(),
            title: "Kinesiology And Movement Studies Explorer".to_string()
        }
    }
}
