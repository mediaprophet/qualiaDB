use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn LandscapePhenomenologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:landscapephenomenology".to_string(),
            title: "Landscape Phenomenology Explorer".to_string()
        }
    }
}
