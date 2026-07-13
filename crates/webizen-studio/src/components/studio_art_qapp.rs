use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn StudioArtQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:studioart".to_string(),
            title: "Studio Art Explorer".to_string()
        }
    }
}
