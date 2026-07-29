use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn LandscapeStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:landscapestudies".to_string(),
            title: "Landscape Studies Explorer".to_string()
        }
    }
}
