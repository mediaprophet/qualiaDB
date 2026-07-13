use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn LandscapePhenomenologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:landscapephenomenology".to_string(),
            title: "Landscape Phenomenology Explorer".to_string()
        }
    }
}
