use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn LandscapeStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:landscapestudies".to_string(),
            title: "Landscape Studies Explorer".to_string()
        }
    }
}
