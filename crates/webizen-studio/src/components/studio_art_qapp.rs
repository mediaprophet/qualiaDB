use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn StudioArtQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:studioart".to_string(),
            title: "Studio Art Explorer".to_string()
        }
    }
}
